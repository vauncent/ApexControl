#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod protocol;
mod backend;
mod ui;
mod desktop;
mod i18n;
mod brand;
mod keyboard;
mod storage;
mod instance;
mod rgb;
mod rgb_input;
mod changelog;
use std::{fs,path::PathBuf};
use protocol::{Profile,MacroEvent};

fn offline_check(root:PathBuf) -> Result<serde_json::Value,String> {
    let captures=root.join("captures");let mut count=0;
    for name in ["actuation-b-18-20-18","rapid-trigger-b-retry"] {
        let reports:serde_json::Value=serde_json::from_slice(&fs::read(captures.join(format!("{name}.control.json"))).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;
        let p=Profile::parse(fs::read(captures.join(format!("{name}.profile-1.bin"))).map_err(|e|e.to_string())?)?;
        for report in reports.as_array().ok_or("report array")? {
            let hex=report["hex"].as_str().ok_or("hex report")?;
            if !hex.starts_with("38") {continue;}
            let bytes:Vec<u8>=(0..hex.len()).step_by(2).map(|i|u8::from_str_radix(&hex[i..i+2],16).unwrap()).collect();
            let sub=bytes[1];let mut expected=p.clone();
            if sub==0x61 && bytes[8]==59 {expected.set_actuation(&[5],20)?;}
            if sub==0x62 && bytes[6]==2 {expected.set_rapid(&[5],true);}
            if sub==0x65 && bytes[6]==3 {expected.set_sensitivity(&[5],3);}
            if expected.live_report(sub)?[1..]!=bytes {return Err(format!("Rust report {sub:02x} differs from GG capture"));}count+=1;
        }
    }
    let original=Profile::parse(fs::read(captures.join("actuation-b-18-20-18.profile-1.bin")).map_err(|e|e.to_string())?)?;
    let golden=Profile::parse(fs::read(captures.join("macro-xy-bind-restore.profile-0.bin")).map_err(|e|e.to_string())?)?;
    let events=vec![MacroEvent::Key{hid:27,down:true},MacroEvent::Delay{ms:78},MacroEvent::Key{hid:27,down:false},MacroEvent::Delay{ms:344},MacroEvent::Key{hid:29,down:true},MacroEvent::Delay{ms:62},MacroEvent::Key{hid:29,down:false}];
    let mut generated=original.clone();generated.assign_first_macro(&[5],&events)?;
    if generated.bytes!=golden.bytes {return Err("Rust macro image differs from captured GG image".into());}
    for slot in 0..5 {Profile::parse(fs::read(root.join(format!("backups/native-slot-{slot}.bin"))).map_err(|e|e.to_string())?)?;}
    Ok(serde_json::json!({"live_reports_match":count,"native_profiles_crc_valid":5,"macro_image_matches_gg":true,"actuation_points":protocol::actuation_points().len()}))
}
fn hardware_check(exercise:bool) -> Result<serde_json::Value,String> {
    let devices=backend::scan()?;let d=devices.iter().find(|d|d.supported).ok_or("No supported Mini connected")?;
    if d.profiles.len()!=5 {return Err(d.error.clone().unwrap_or("Could not read five profiles".into()));}
    let api=hidapi::HidApi::new().map_err(|e|e.to_string())?;let device=api.open_path(&d.path).map_err(|e|e.to_string())?;
    let original=backend::read_profile(&device,2)?;let mut live_ok=false;let mut protection_ok=false;let mut flash_ok=false;let mut macro_ok=false;
    if exercise {
        if d.firmware!="1.19.7" {return Err(format!("Self-test requires investigated firmware 1.19.7, got {}",d.firmware));}
        let result=(||{
            let mut changed=original.clone();changed.set_actuation(&[5],20)?;changed.set_rapid(&[5],true);changed.set_sensitivity(&[5],3);backend::apply_live(&device,&changed)?;backend::apply_live(&device,&original)?;live_ok=true;
            changed=original.clone();changed.set_protection(&[5],true);backend::apply_live(&device,&changed)?;backend::apply_live(&device,&original)?;protection_ok=true;
            backend::write_profile(&device,2,&original,&original)?;flash_ok=true;
            let events=vec![MacroEvent::Key{hid:27,down:true},MacroEvent::Delay{ms:78},MacroEvent::Key{hid:27,down:false},MacroEvent::Delay{ms:344},MacroEvent::Key{hid:29,down:true},MacroEvent::Delay{ms:62},MacroEvent::Key{hid:29,down:false}];
            let mut macro_profile=original.clone();macro_profile.assign_first_macro(&[5],&events)?;
            backend::write_profile(&device,2,&macro_profile,&original)?;
            backend::write_profile(&device,2,&original,&macro_profile)?;macro_ok=true;Ok::<_,String>(())
        })();
        if let Err(error)=result {
            let _=backend::apply_live(&device,&original);
            if let Ok(current)=backend::read_profile(&device,2) {if current!=original {let _=backend::write_profile(&device,2,&original,&current);}}
            return Err(error);
        }
    }
    Ok(serde_json::json!({"device":d.name,"firmware":d.firmware,"region_id":d.region_id,"physical_layout_id":d.layout_id,"detected_keyboard_layout":d.detected_layout,"onboard_profiles":d.profiles.iter().map(Profile::name).collect::<Vec<_>>(),"live_applied_and_restored":live_ok,"protection_accepted_and_restored":protection_ok,"persistent_write_readback_verified":flash_ok,"native_macro_write_readback_restored":macro_ok,"original_profile_intact":backend::read_profile(&device,2)?==original}))
}
fn main() -> eframe::Result {
    let args:Vec<String>=std::env::args().collect();
    if args.iter().any(|a|a=="--offline-check" || a=="--hardware-check" || a=="--exercise-hardware" || a=="--rgb-check") {
        let result=if args.iter().any(|a|a=="--rgb-check") {rgb_check()} else if let Some(i)=args.iter().position(|a|a=="--offline-check") {offline_check(PathBuf::from(args.get(i+1).expect("reference directory")))} else {hardware_check(args.iter().any(|a|a=="--exercise-hardware"))};
        let value=match result {Ok(result)=>serde_json::json!({"success":true,"result":result}),Err(error)=>serde_json::json!({"success":false,"error":error})};
        let text=serde_json::to_string_pretty(&value).unwrap();println!("{text}");
        if let Some(i)=args.iter().position(|a|a=="--report") {
            let Some(path)=args.get(i+1) else {eprintln!("Missing report path");std::process::exit(2)};
            if let Err(error)=fs::write(path,&text) {eprintln!("Cannot write report: {error}");std::process::exit(2);}
        }
        if value["success"]!=true {std::process::exit(1);}return Ok(());
    }
    let Some(mut instance)=instance::Guard::acquire(!args.iter().any(|arg|arg=="--autostart")).map_err(|error|eframe::Error::AppCreation(Box::new(error)))? else{return Ok(())};
    let icon=eframe::egui::IconData{rgba:desktop::icon_rgba(),width:32,height:32};
    let input=rgb_input::Input::default();
    let mut options=eframe::NativeOptions { viewport:eframe::egui::ViewportBuilder::default().with_icon(icon).with_inner_size([1320.0,850.0]).with_min_inner_size([1100.0,740.0]), ..Default::default() };
    rgb_input::install(&mut options,input.clone());
    eframe::run_native("ApexControl",options,Box::new(move|cc|{instance.attach(cc)?;Ok(Box::new(ui::ApexApp::new(cc,instance,input)))}))
}

fn rgb_check() -> Result<serde_json::Value,String> {
    let devices=backend::scan()?;let d=devices.iter().find(|d|d.supported).ok_or("No supported Mini connected")?;
    if d.firmware!="1.19.7" {return Err("RGB check requires investigated firmware 1.19.7".into());}
    let api=hidapi::HidApi::new().map_err(|e|e.to_string())?;let device=api.open_path(&d.path).map_err(|e|e.to_string())?;
    let original=backend::read_profile(&device,2)?;
    let result=backend::apply_rgb(&device,&[5],[20,150,50]);
    if result.is_ok() {std::thread::sleep(std::time::Duration::from_secs(8));}
    let restored=backend::clear_rgb(&device);
    result?;restored?;
    if backend::read_profile(&device,2)?!=original {return Err("Unexpected profile change during RGB check".into());}
    Ok(serde_json::json!({"firmware":d.firmware,"rgb_command_acknowledged":true,"onboard_lighting_restore_sent":true,"stored_profile_unchanged":true}))
}

