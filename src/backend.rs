use crate::protocol::{Profile, PID_MINI, PROFILE_LEN};
use hidapi::{HidApi,HidDevice};
use serde::{Deserialize,Serialize};
use std::{collections::BTreeMap, ffi::CString, fs, path::PathBuf, sync::mpsc, thread, time::{Duration,Instant,SystemTime,UNIX_EPOCH}};

#[derive(Clone)]
pub struct Device {
    pub id:String, pub name:String, pub pid:u16, pub path:CString, pub supported:bool,
    pub profiles:Vec<Profile>, pub firmware:String, pub error:Option<String>,
    pub region_id:Option<u8>,pub layout_id:Option<u8>,pub detected_layout:Option<crate::keyboard::Layout>,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct SavedProfile { pub format:u8, pub name:String, pub pid:u16, pub firmware:String, pub profile:Profile }
#[derive(Clone)]
pub struct Backup {pub file_name:String,pub slot:Option<u8>,pub modified:Option<SystemTime>,pub profile:Profile}
#[cfg(test)]
thread_local!{static TEST_DATA_DIR:std::cell::RefCell<Option<PathBuf>>=const{std::cell::RefCell::new(None)};}
#[cfg(test)]
pub fn with_data_dir<R>(folder:PathBuf,body:impl FnOnce()->R)->R{
    struct Reset(Option<PathBuf>);impl Drop for Reset{fn drop(&mut self){TEST_DATA_DIR.with(|value|*value.borrow_mut()=self.0.take());}}
    let _reset=Reset(TEST_DATA_DIR.with(|value|value.replace(Some(folder))));body()
}
pub fn backups()->Result<(Vec<Backup>,usize),String>{read_backups(&data_dir().join("backups"))}
fn read_backups(folder:&std::path::Path)->Result<(Vec<Backup>,usize),String>{
    let files=match fs::read_dir(folder){Ok(files)=>files,Err(e) if e.kind()==std::io::ErrorKind::NotFound=>return Ok((Vec::new(),0)),Err(e)=>return Err(e.to_string())};
    let mut entries=Vec::new();let mut rejected=0;
    for file in files{
        let file=match file{Ok(file)=>file,Err(_)=>{rejected+=1;continue;}};
        if !file.path().extension().and_then(|ext|ext.to_str()).is_some_and(|ext|ext.eq_ignore_ascii_case("bin")){continue;}
        let profile=crate::storage::read(&file.path(),PROFILE_LEN).map_err(|e|e.to_string()).and_then(Profile::parse);
        match profile{Ok(profile)=>{
            let file_name=file.file_name().to_string_lossy().into_owned();
            let slot=file_name.strip_prefix("slot-").and_then(|rest|rest.split('-').next()).and_then(|slot|slot.parse::<u8>().ok()).filter(|slot|(1..=5).contains(slot)).map(|slot|slot-1);
            entries.push(Backup{file_name,slot,modified:file.metadata().ok().and_then(|metadata|metadata.modified().ok()),profile});
        },Err(_)=>rejected+=1}
    }
    entries.sort_by(|a,b|b.modified.cmp(&a.modified).then_with(||b.file_name.cmp(&a.file_name)));
    Ok((entries,rejected))
}
pub fn data_dir() -> PathBuf {
    #[cfg(test)] if let Some(folder)=TEST_DATA_DIR.with(|value|value.borrow().clone()){return folder;}
    std::env::var_os("APEXCONTROL_DATA_DIR").map(PathBuf::from).unwrap_or_else(||std::env::var_os("LOCALAPPDATA").map(PathBuf::from).unwrap_or_else(||PathBuf::from(".")).join("ApexControl"))
}
pub fn saved_profiles() -> Vec<SavedProfile> {
    let folder=data_dir().join("profiles"); let Ok(files)=fs::read_dir(folder) else {return Vec::new()};
    let mut profiles=Vec::new();
    for file in files.flatten().filter(|file|file.path().extension().and_then(|ext|ext.to_str()).is_some_and(|ext|ext.eq_ignore_ascii_case("json"))) { if let Ok(bytes)=crate::storage::read(&file.path(),crate::storage::PROFILE_LIMIT) { if let Ok(profile)=serde_json::from_slice::<SavedProfile>(&bytes) {
        if profile.format==1 && profile.pid==PID_MINI && Profile::parse(profile.profile.bytes.clone()).is_ok() { profiles.push(profile); }
    } } }
    profiles.sort_by(|a,b|a.name.to_lowercase().cmp(&b.name.to_lowercase()));profiles
}
pub fn save_local(name:&str,profile:&Profile,firmware:&str) -> Result<(),String> {
    if name.trim().is_empty() {return Err(crate::i18n::text("Bitte einen Profilnamen eingeben").into());}
    Profile::parse(profile.bytes.clone())?;
    let saved=SavedProfile{format:1,name:name.trim().into(),pid:PID_MINI,firmware:firmware.into(),profile:profile.clone()};
    let folder=data_dir().join("profiles");fs::create_dir_all(&folder).map_err(|e|e.to_string())?;
    let safe:String=name.chars().map(|c|if c.is_alphanumeric() || " -_".contains(c) {c} else {'_'}).collect();
    let path=folder.join(format!("{}.json",safe.chars().take(70).collect::<String>()));
    crate::storage::write(&path,&serde_json::to_vec_pretty(&saved).map_err(|e|e.to_string())?,crate::storage::PROFILE_LIMIT).map_err(|e|e.to_string())
}
pub fn load_macros()->Vec<crate::protocol::MacroRecord>{
    crate::storage::read(&data_dir().join("macros.json"),crate::storage::LIBRARY_LIMIT).ok()
        .and_then(|bytes|serde_json::from_slice::<Vec<crate::protocol::MacroRecord>>(&bytes).ok())
        .unwrap_or_default().into_iter().filter(|record|record.events.len()<=1000).collect()
}
pub fn save_macro(library:&[crate::protocol::MacroRecord],entry:&crate::protocol::MacroRecord)->Result<(),String>{
    if entry.events.len()>1000{return Err(crate::i18n::text("Das Makro hat die maximale Länge erreicht.").into());}
    let entries:Vec<_>=library.iter().chain(std::iter::once(entry)).collect();
    crate::storage::write(&data_dir().join("macros.json"),&serde_json::to_vec_pretty(&entries).map_err(|e|e.to_string())?,crate::storage::LIBRARY_LIMIT).map_err(|e|e.to_string())
}

fn feature(device:&HidDevice,payload:&[u8]) -> Result<(),String> {
    let mut report=vec![0;513]; if payload.len()>512 {return Err(crate::i18n::text("Ungültige Befehlslänge").into());}
    report[1..1+payload.len()].copy_from_slice(payload); device.send_feature_report(&report).map_err(|e|e.to_string())
}
fn feature_reply(device:&HidDevice) -> Result<Vec<u8>,String> {
    let mut reply=vec![0;513];let length=device.get_feature_report(&mut reply).map_err(|e|e.to_string())?;
    decode_feature_reply(&reply,length)
}
fn decode_feature_reply(reply:&[u8],length:usize)->Result<Vec<u8>,String>{
    if length<3 || length>reply.len() || reply[0]!=0 {return Err(crate::i18n::text("Die Tastatur hat unvollständig geantwortet").into());}
    Ok(reply[1..length].to_vec())
}
fn output_report(command:&[u8])->Result<[u8;65],String>{
    if command.len()>64{return Err(crate::i18n::text("Ungültige Befehlslänge").into());}
    let mut report=[0u8;65];report[1..1+command.len()].copy_from_slice(command);Ok(report)
}
fn output(device:&HidDevice,command:&[u8]) -> Result<(),String> {
    let report=output_report(command)?;
    if device.write(&report).map_err(|e|e.to_string())?!=report.len(){return Err(crate::i18n::text("Die Tastatur hat unvollständig geantwortet").into());}Ok(())
}
fn wait_reply(device:&HidDevice,opcode:u8,timeout:Duration) -> Result<Vec<u8>,String> {
    let start=Instant::now();
    loop {
        if let Ok(reply)=feature_reply(device) { if reply[0]==opcode { if reply[1]!=0 {return Err(crate::i18n::format_text("Tastatur meldet Fehler {} bei {opcode:02X}", &[format!("{}", reply[1]), format!("{:02X}", opcode)]));} return Ok(reply); } }
        if start.elapsed()>timeout {return Err(crate::i18n::format_text("Die Tastatur antwortet nicht auf {opcode:02X}. Bitte GG schließen und neu laden.", &[format!("{:02X}", opcode)]));}
        thread::sleep(Duration::from_millis(15));
    }
}
fn wait_input(device:&HidDevice,opcode:u8,timeout:Duration) -> Result<Vec<u8>,String> {
    let start=Instant::now();
    while start.elapsed()<timeout {
        let mut report=[0u8;65];
        let length=device.read_timeout(&mut report,100).map_err(|e|e.to_string())?;
        let reply=if length==65 && report[0]==0 {&report[1..length]} else {&report[..length]};
        if reply.len()>=2 && reply[0]==opcode {
            if reply[1]!=0 {return Err(crate::i18n::format_text("Tastatur meldet Fehler {} bei {opcode:02X}", &[format!("{}", reply[1]), format!("{:02X}", opcode)]));}
            return Ok(reply.to_vec());
        }
    }
    Err(crate::i18n::format_text("Keine Geräteantwort auf {opcode:02X}. Bitte GG vollständig schließen.", &[format!("{:02X}", opcode)]))
}
pub fn read_profile(device:&HidDevice,slot:u8) -> Result<Profile,String> {
    if slot>4 {return Err(crate::i18n::text("Ungültiger Onboard-Slot").into());}
    let mut bytes=Vec::with_capacity(PROFILE_LEN);
    for offset in (0..PROFILE_LEN).step_by(500) {
        let mut request=[0u8;512];request[..3].copy_from_slice(&[0x83,1,0x80+slot]);request[3..5].copy_from_slice(&500u16.to_le_bytes());request[5..9].copy_from_slice(&(offset as u32).to_le_bytes());
        let mut chunk=None;
        for _ in 0..3 {
            feature(device,&request)?;thread::sleep(Duration::from_millis(18));
            let reply=feature_reply(device)?;
            if let Some(data)=profile_chunk(&reply) {chunk=Some(data.to_vec());break;}
            thread::sleep(Duration::from_millis(30));
        }
        bytes.extend(chunk.ok_or(crate::i18n::text("Onboard-Profil konnte nicht vollständig gelesen werden. Bitte GG schließen."))?);
    }
    Profile::parse(bytes)
}
pub fn activate(device:&HidDevice,slot:u8) -> Result<(),String> {
    if slot>4 {return Err(crate::i18n::text("Ungültiger Onboard-Slot").into());}
    output(device,&[0xb2,slot])?;wait_input(device,0xb2,Duration::from_secs(2))?;Ok(())
}
pub fn apply_live(device:&HidDevice,profile:&Profile) -> Result<(),String> {
    Profile::parse(profile.bytes.clone())?;
    for sub in [0x61,0x62,0x65] {
        let report=profile.live_report(sub)?;device.send_feature_report(&report).map_err(|e|e.to_string())?;
        thread::sleep(Duration::from_millis(20));let reply=wait_reply(device,0x38,Duration::from_secs(2))?;
        if reply.get(2)!=Some(&sub) {return Err(crate::i18n::text("Antwort gehört zu einer anderen Einstellung. Bitte GG schließen.").into());}
    }Ok(())
}
pub fn apply_rgb(device:&HidDevice,keys:&[u8],color:[u8;3]) -> Result<(),String> {
    let keys:std::collections::BTreeSet<_>=keys.iter().copied().collect();
    apply_rgb_frame(device,&keys.into_iter().map(|key|(key,color)).collect())
}
fn apply_rgb_frame(device:&HidDevice,frame:&crate::rgb::Frame)->Result<(),String>{
    feature(device,&crate::rgb::report(frame)?)?;
    let _=wait_reply(device,0x40,Duration::from_secs(2))?;Ok(())
}
pub fn clear_rgb(device:&HidDevice) -> Result<(),String> {
    output(device,&[0x41])?;thread::sleep(Duration::from_millis(40));Ok(())
}
pub fn write_profile(device:&HidDevice,slot:u8,profile:&Profile,expected:&Profile) -> Result<(),String> {
    if slot>4 {return Err(crate::i18n::text("Ungültiger Onboard-Slot").into());}
    Profile::parse(profile.bytes.clone())?;
    let before=read_profile(device,slot)?;
    if before.bytes!=expected.bytes {return Err(crate::i18n::text("Das Onboard-Profil wurde inzwischen geändert. Bitte zuerst neu laden.").into());}
    let folder=data_dir().join("backups");fs::create_dir_all(&folder).map_err(|e|e.to_string())?;
    let stamp=SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();
    crate::storage::write(&folder.join(format!("slot-{}-{stamp}.bin",slot+1)),&before.bytes,crate::storage::PROFILE_LIMIT).map_err(|e|e.to_string())?;
    fn transfer(device:&HidDevice,slot:u8,profile:&Profile) -> Result<(),String> {
        output(device,&[2,1,0x80+slot])?;wait_input(device,2,Duration::from_secs(6))?;
        for offset in (0..PROFILE_LEN).step_by(500) {
            let mut packet=vec![3,1,0x80+slot];packet.extend(500u16.to_le_bytes());packet.extend((offset as u32).to_le_bytes());packet.extend(&profile.bytes[offset..offset+500]);packet.resize(512,0);
            feature(device,&packet)?;thread::sleep(Duration::from_millis(20));
        }
        output(device,&[0xb1,slot])?;wait_input(device,0xb1,Duration::from_secs(3))?;
        let verify=read_profile(device,slot)?;
        if verify.bytes!=profile.bytes {return Err(crate::i18n::text("Die Rückleseprüfung des gespeicherten Profils ist fehlgeschlagen").into());}Ok(())
    }
    if let Err(error)=transfer(device,slot,profile) {
        return match transfer(device,slot,&before) {
            Ok(())=>Err(crate::i18n::format_text("Speichern fehlgeschlagen; das ursprüngliche Profil wurde wiederhergestellt. {error}", &[format!("{}", error)])),
            Err(restore)=>Err(crate::i18n::format_text("Speichern und Wiederherstellung fehlgeschlagen. Sicherung liegt unter {}. {error}; {restore}", &[format!("{}", folder.display()), format!("{}", error), format!("{}", restore)]))
        };
    }
    activate(device,slot)?;Ok(())
}

fn model_name(pid:u16,reported:Option<&str>) -> String {
    match pid {0x1610=>"Apex Pro",0x1612=>"Apex 7",0x1614=>"Apex Pro TKL",0x1616=>"Apex 150",0x1618=>"Apex 7 TKL",0x161a=>"Apex 3",0x161c=>"Apex 5",0x161e=>"Apex Pro Mini",0x1620=>"Apex 9 Mini",0x1622=>"Apex 3 TKL",0x1624|0x1626=>"Apex Pro Mini Wireless",0x1628=>"Apex Pro TKL (2023)",0x1630=>"Apex Pro TKL Wireless",0x1640=>"Apex Pro Gen 3",0x1642=>"Apex Pro TKL Gen 3",0x1644|0x1646=>"Apex Pro TKL Wireless Gen 3",0x1648=>"Apex Pro Mini Gen 3",_=>reported.unwrap_or("Apex Keyboard")}.replace("SteelSeries ","")
}
pub fn scan() -> Result<Vec<Device>,String> {
    let api=HidApi::new().map_err(|e|e.to_string())?;let mut groups:BTreeMap<String,Device>=BTreeMap::new();
    for info in api.device_list() {
        if info.vendor_id()!=0x1038 {continue;}
        let product=info.product_string().unwrap_or("");
        if !product.to_lowercase().contains("apex") && !(0x1610..=0x164f).contains(&info.product_id()) {continue;}
        let id=format!("{:04x}-{}",info.product_id(),info.serial_number().unwrap_or("wired"));
        let entry=groups.entry(id.clone()).or_insert_with(||Device{id,name:model_name(info.product_id(),info.product_string()),pid:info.product_id(),path:info.path().to_owned(),supported:false,profiles:Vec::new(),firmware:String::new(),error:None,region_id:None,layout_id:None,detected_layout:None});
        if info.usage_page()==0xffc0 && info.usage()==1 {entry.path=info.path().to_owned();entry.supported=info.product_id()==PID_MINI;}
    }
    let mut devices:Vec<_>=groups.into_values().collect();
    for device in &mut devices {
        if !device.supported {continue;}
        let result=(||{
            let hid=open_control(&device.path)?;
            output(&hid,&[0x90,0])?;
            let reply=wait_input(&hid,0x90,Duration::from_secs(2))?;
            device.firmware=String::from_utf8_lossy(&reply[2..].split(|b|*b==0).next().unwrap_or_default()).into_owned();
            // Identity queries only. Never send the region-write opcode 0x75.
            fn identity(hid:&HidDevice,opcode:u8)->Option<u8>{output(hid,&[opcode]).ok()?;wait_input(hid,opcode,Duration::from_millis(600)).ok()?.get(2).copied()}
            device.region_id=identity(&hid,0xf5);device.layout_id=identity(&hid,0xf2);
            device.detected_layout=crate::keyboard::Layout::from_device(device.region_id,device.layout_id);
            for slot in 0..5 {device.profiles.push(read_profile(&hid,slot)?);}Ok::<_,String>(())
        })();
        if let Err(error)=result {device.error=Some(error);}
    }Ok(devices)
}

pub enum Request {
    Scan,
    Load{path:CString,slot:u8},
    Apply{path:CString,profile:Profile},
    Store{path:CString,slot:u8,profile:Profile,expected:Profile},
    Rgb{path:CString,keys:Vec<u8>,color:[u8;3]},
    RgbEffect{path:CString,device_id:String,config:crate::rgb::Config,preview:crate::rgb::Preview},
    ClearRgb{path:CString},
}
pub enum Response { Scanned(Result<Vec<Device>,String>), Loaded(Result<(u8,Profile),String>), Applied(Result<(),String>), Stored(Result<(u8,Profile),String>), Rgb(Result<(),String>), RgbStarted(Result<(),String>),RgbError(String), ClearedRgb(Result<(),String>) }
fn profile_chunk(reply:&[u8])->Option<&[u8]>{
    if reply.get(..2)!=Some(&[0x83,0][..]){return None;}reply.get(2..502)
}
fn supported_interface(vendor:u16,product:u16,page:u16,usage:u16)->bool{
    vendor==0x1038 && product==PID_MINI && page==0xffc0 && usage==1
}
fn open_control(path:&CString)->Result<HidDevice,String>{
    let api=HidApi::new().map_err(|e|e.to_string())?;
    let device=api.open_path(path).map_err(|e|e.to_string())?;
    // Recheck the actual opened handle, rather than trusting a previous scan.
    let info=device.get_device_info().map_err(|e|e.to_string())?;
    if !supported_interface(info.vendor_id(),info.product_id(),info.usage_page(),info.usage()){
        return Err(crate::i18n::text("Diese Geräteschnittstelle wird nicht unterstützt").into());
    }Ok(device)
}
pub fn worker(input:crate::rgb_input::Input) -> (mpsc::Sender<Request>,mpsc::Receiver<Response>) {
    let (tx,rx)=mpsc::channel();let (out,events)=mpsc::channel();
    thread::spawn(move||{
        struct Running{device:HidDevice,id:String,effect:crate::rgb::Effect,preview:crate::rgb::Preview,started:Instant,last:Instant}
        let mut lighting:Option<Running>=None;
        loop {
            let request=if lighting.is_some(){match rx.recv_timeout(Duration::from_millis(33)){Ok(request)=>Some(request),Err(mpsc::RecvTimeoutError::Timeout)=>None,Err(_)=>break}}else{match rx.recv(){Ok(request)=>Some(request),Err(_)=>break}};
            if let Some(request)=request{
            let response=match request {
                Request::Scan=>Response::Scanned(scan()),
                Request::Load{path,slot}=>Response::Loaded((||{let dev=open_control(&path)?;let profile=read_profile(&dev,slot)?;activate(&dev,slot)?;Ok((slot,profile))})()),
                Request::Apply{path,profile}=>Response::Applied((||{let dev=open_control(&path)?;apply_live(&dev,&profile)})()),
                Request::Store{path,slot,profile,expected}=>Response::Stored((||{let dev=open_control(&path)?;write_profile(&dev,slot,&profile,&expected)?;Ok((slot,profile))})()),
                Request::Rgb{path,keys,color}=>{lighting=None;input.disable();Response::Rgb((||{let dev=open_control(&path)?;apply_rgb(&dev,&keys,color)})())},
                Request::RgbEffect{path,device_id,config,preview}=>{
                    lighting=None;input.disable();
                    let result=(||{let mut effect=crate::rgb::Effect::new(config)?;let device=open_control(&path)?;
                        if effect.config.mode==crate::rgb::Mode::Ripple{input.enable()?;}
                        let frame=effect.frame(0.0);apply_rgb_frame(&device,&frame)?;
                        if let Ok(mut latest)=preview.lock(){*latest=Some((device_id.clone(),frame));}
                        lighting=Some(Running{device,id:device_id,effect,preview,started:Instant::now(),last:Instant::now()});Ok(())
                    })();if result.is_err(){input.disable();}Response::RgbStarted(result)
                },
                Request::ClearRgb{path}=>{lighting=None;input.disable();Response::ClearedRgb((||{let dev=open_control(&path)?;clear_rgb(&dev)})())},
            };if out.send(response).is_err() {break;}
            }
            if lighting.as_ref().is_some_and(|state|state.last.elapsed()>=Duration::from_millis(33)){
                let state=lighting.as_mut().unwrap();let time=state.started.elapsed().as_secs_f32();
                for hid in input.drain(){state.effect.pulse(hid,time);}
                let frame=state.effect.frame(time);
                match apply_rgb_frame(&state.device,&frame){Ok(())=>{state.last=Instant::now();if let Ok(mut latest)=state.preview.lock(){*latest=Some((state.id.clone(),frame));}},Err(error)=>{lighting=None;input.disable();if out.send(Response::RgbError(error)).is_err(){break;}}}
            }
        }
        input.disable();
    });(tx,events)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn backup_catalog_checks_crc_size_and_slot_and_ignores_other_files(){
        let folder=std::env::temp_dir().join(format!("apexcontrol-backup-test-{}",std::process::id()));fs::create_dir_all(&folder).unwrap();
        let mut profile=Profile{bytes:vec![0;PROFILE_LEN]};profile.bytes[0]=3;profile.bytes[780]=6;profile.set_name("Original");
        fs::write(folder.join("slot-3-100.bin"),&profile.bytes).unwrap();fs::write(folder.join("slot-0-100.bin"),&profile.bytes).unwrap();
        let mut corrupt=profile.bytes.clone();corrupt[100]^=1;fs::write(folder.join("slot-2-corrupt.bin"),corrupt).unwrap();
        fs::write(folder.join("oversized.bin"),vec![0;PROFILE_LEN+1]).unwrap();fs::write(folder.join("readme.txt"),b"ignored").unwrap();
        let (entries,rejected)=read_backups(&folder).unwrap();assert_eq!(entries.len(),2);assert_eq!(rejected,2);
        assert!(entries.iter().any(|backup|backup.slot==Some(2)));assert!(entries.iter().any(|backup|backup.slot.is_none()));
        assert!(entries.iter().all(|backup|backup.profile==profile));
        assert!(folder.starts_with(std::env::temp_dir()));fs::remove_dir_all(folder).unwrap();
    }
    #[test]
    fn short_usb_replies_are_not_zero_padded_into_success(){
        let mut buffer=[0;513];buffer[1]=0x83;
        assert!(decode_feature_reply(&buffer,2).is_err());
        let reply=decode_feature_reply(&buffer,3).unwrap();
        assert_eq!(reply,vec![0x83,0]);assert!(profile_chunk(&reply).is_none());
        assert!(profile_chunk(&decode_feature_reply(&buffer,502).unwrap()).is_none());
        assert_eq!(profile_chunk(&decode_feature_reply(&buffer,503).unwrap()).unwrap().len(),500);
        buffer[1]=0x38;assert!(profile_chunk(&decode_feature_reply(&buffer,513).unwrap()).is_none());
        assert!(decode_feature_reply(&buffer,514).is_err());buffer[0]=1;
        assert!(decode_feature_reply(&buffer,513).is_err());
    }
    #[test]
    fn report_boundaries_and_device_identity_are_enforced(){
        assert!(output_report(&[0;65]).is_err());assert_eq!(output_report(&[7;64]).unwrap()[1..],[7;64]);
        assert!(supported_interface(0x1038,PID_MINI,0xffc0,1));
        for (vendor,product,page,usage) in [(0x1234,PID_MINI,0xffc0,1),(0x1038,0x1614,0xffc0,1),(0x1038,PID_MINI,1,6),(0x1038,PID_MINI,0xffc0,2)]{
            assert!(!supported_interface(vendor,product,page,usage));
        }
    }
}

