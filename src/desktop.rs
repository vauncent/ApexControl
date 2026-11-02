use eframe::egui;
use serde::{Deserialize,Serialize};
use std::{path::Path,sync::mpsc};
#[cfg(test)] use std::fs;

#[derive(Clone,Default,PartialEq,Serialize,Deserialize)]
#[serde(default)]
pub struct Settings {
    pub minimize_on_close:bool,
    pub autostart_minimized:bool,
    pub language:crate::i18n::Language,
    pub language_setup_done:bool,
    pub layout_setup_done:bool,
    pub keyboard_layout:Option<crate::keyboard::Layout>,
}
impl Settings {
    pub fn load()->Self {
        let mut settings:Self=crate::storage::read(&crate::backend::data_dir().join("settings.json"),crate::storage::SETTINGS_LIMIT).ok()
            .and_then(|bytes|serde_json::from_slice(&bytes).ok()).unwrap_or_default();
        if settings.language_setup_done && settings.keyboard_layout.is_some(){settings.layout_setup_done=true;}settings
    }
    pub fn setup_complete(&self)->bool{self.language_setup_done && self.layout_setup_done}
    pub fn save(&self)->Result<(),String> {self.save_in(&crate::backend::data_dir())}
    pub fn confirm_setup(&mut self,language:crate::i18n::Language,layout:crate::keyboard::Layout)->Result<(),String>{self.confirm_setup_in(language,layout,&crate::backend::data_dir())}
    fn confirm_setup_in(&mut self,language:crate::i18n::Language,layout:crate::keyboard::Layout,folder:&Path)->Result<(),String>{
        let mut confirmed=self.clone();confirmed.language=language;confirmed.keyboard_layout=Some(layout);confirmed.language_setup_done=true;confirmed.layout_setup_done=true;
        confirmed.save_in(folder)?;*self=confirmed;Ok(())
    }
    fn save_in(&self,folder:&Path)->Result<(),String> {
        crate::storage::write(&folder.join("settings.json"),&serde_json::to_vec_pretty(self).map_err(|error|error.to_string())?,crate::storage::SETTINGS_LIMIT).map_err(|error|error.to_string())
    }
}

// Quote the executable, including paths with spaces; no shell is involved.
fn autostart_command(executable:&Path)->Result<String,String> {
    let path=executable.to_str().ok_or(crate::i18n::text("Der Programmpfad ist kein gültiger Text"))?;
    if path.contains(['"','\r','\n']) {return Err("Ungültiger Programmpfad".into());}
    Ok(format!("\"{path}\" --autostart"))
}

#[cfg(windows)]
const RUN_KEY:&str=r"Software\Microsoft\Windows\CurrentVersion\Run";
#[cfg(windows)]
const APPROVED_KEY:&str=r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run";
#[cfg(windows)]
pub fn autostart_enabled()->Result<bool,String> {
    let key=match winreg::HKCU.open_subkey(RUN_KEY) {
        Ok(key)=>key,Err(error) if error.kind()==std::io::ErrorKind::NotFound=>return Ok(false),Err(error)=>return Err(error.to_string()),
    };
    let approved=match winreg::HKCU.open_subkey(APPROVED_KEY){Ok(key)=>Some(key),Err(e) if e.kind()==std::io::ErrorKind::NotFound=>None,Err(e)=>return Err(e.to_string())};
    read_autostart(&key,approved.as_ref(),&autostart_command(&std::env::current_exe().map_err(|error|error.to_string())?)?)
}
#[cfg(windows)]
fn read_autostart(key:&winreg::RegKey,approved:Option<&winreg::RegKey>,command:&str)->Result<bool,String>{
    let value:String=match key.get_value("ApexControl") {
        Ok(value)=>value,Err(error) if error.kind()==std::io::ErrorKind::NotFound=>return Ok(false),Err(error)=>return Err(error.to_string()),
    };
    if value!=command{return Ok(false);}
    let Some(approved)=approved else{return Ok(true)};
    let value=match approved.get_raw_value("ApexControl"){Ok(value)=>value,Err(e) if e.kind()==std::io::ErrorKind::NotFound=>return Ok(true),Err(e)=>return Err(e.to_string())};
    if value.vtype!=winreg::enums::REG_BINARY || value.bytes.len()!=12{
        return Err(crate::i18n::text("Der Windows-Autostartstatus ist ungültig").into());
    }
    Ok(matches!(u32::from_le_bytes(value.bytes[..4].try_into().unwrap()),0|2))
}
#[cfg(windows)]
pub fn set_autostart(enabled:bool)->Result<(),String> {
    let executable=std::env::current_exe().map_err(|error|error.to_string())?;
    let command=autostart_command(&executable)?;
    let (key,_)=winreg::HKCU.create_subkey(RUN_KEY).map_err(|error|error.to_string())?;
    let (approved,_)=winreg::HKCU.create_subkey(APPROVED_KEY).map_err(|error|error.to_string())?;
    update_autostart(&key,&approved,enabled,&command)
}
#[cfg(windows)]
fn update_autostart(key:&winreg::RegKey,approved:&winreg::RegKey,enabled:bool,command:&str)->Result<(),String> {
    let previous=match key.get_raw_value("ApexControl"){Ok(value)=>Some(value),Err(e) if e.kind()==std::io::ErrorKind::NotFound=>None,Err(e)=>return Err(e.to_string())};
    // Keep the registration when disabled so Task Manager can enable it again.
    // Only change our own approval value on an explicit user toggle, never on
    // startup or during polling of changes made in Task Manager.
    key.set_value("ApexControl",&command).map_err(|error|error.to_string())?;
    let mut bytes=vec![0;12];bytes[0]=if enabled{2}else{3};
    if let Err(error)=approved.set_raw_value("ApexControl",&winreg::RegValue{vtype:winreg::enums::REG_BINARY,bytes:bytes.into()}){
        let rollback=if let Some(previous)=previous{key.set_raw_value("ApexControl",&previous)}else{key.delete_value("ApexControl")};
        return Err(match rollback{Ok(())=>error.to_string(),Err(restore)=>format!("{error}; rollback: {restore}")});
    }
    Ok(())
}
#[cfg(not(windows))]
pub fn autostart_enabled()->Result<bool,String> {Ok(false)}
#[cfg(not(windows))]
pub fn set_autostart(_:bool)->Result<(),String> {Err(crate::i18n::text("Autostart ist bisher nur unter Windows verfügbar").into())}

#[derive(Clone,Copy,Debug,PartialEq)]
pub enum Action {Show,Settings,Quit}

#[cfg(windows)]
#[derive(Clone,Copy)]
struct NativeWindow(isize);
#[cfg(windows)]
impl NativeWindow {
    fn from_creation_context(cc:&eframe::CreationContext<'_>)->Result<Self,String> {
        use raw_window_handle::{HasWindowHandle,RawWindowHandle};
        match cc.window_handle().map_err(|error|error.to_string())?.as_raw() {
            RawWindowHandle::Win32(handle)=>Ok(Self(handle.hwnd.get())),
            _=>Err(crate::i18n::text("Das Windows-Fenster konnte nicht ermittelt werden").into()),
        }
    }
    fn restore(self,focus:bool)->bool {
        use windows_sys::Win32::UI::WindowsAndMessaging::{GetWindowThreadProcessId,IsWindow,IsWindowVisible,IsIconic,ShowWindow,SetForegroundWindow,SW_RESTORE,SW_SHOW};
        let window=self.0 as windows_sys::Win32::Foundation::HWND;
        // The handle comes from our own eframe window. Validate ownership on
        // every call: the global tray callback can outlive the window at exit.
        unsafe {
            if IsWindow(window)==0 {return false;}
            let mut owner=0;GetWindowThreadProcessId(window,&mut owner);
            if owner!=std::process::id() {return false;}
            ShowWindow(window,if IsIconic(window)!=0{SW_RESTORE}else{SW_SHOW});
            if focus {SetForegroundWindow(window);}
            IsWindowVisible(window)!=0
        }
    }
    fn wake(self,ctx:&egui::Context) {
        // A hidden Win32 window need not receive WM_PAINT. Restore it here,
        // before queuing egui commands, so App::update can run again.
        self.restore(true);show_window(ctx);
    }
}

pub struct Desktop {
    #[cfg(windows)]
    _icon:tray_icon::TrayIcon,
    #[cfg(windows)]
    labels:[tray_icon::menu::MenuItem;3],
    events:mpsc::Receiver<Action>,
}
impl Desktop {
    #[cfg(windows)]
    pub fn new(cc:&eframe::CreationContext<'_>)->Result<Self,String> {
        use tray_icon::{TrayIconBuilder,Icon,TrayIconEvent,MouseButton,MouseButtonState,menu::{Menu,MenuItem,MenuEvent,PredefinedMenuItem}};
        let ctx=&cc.egui_ctx;let window=NativeWindow::from_creation_context(cc)?;
        let menu=Menu::new();
        let show=MenuItem::new(crate::i18n::text("ApexControl öffnen"),true,None);
        let settings=MenuItem::new(crate::i18n::text("Einstellungen"),true,None);
        let quit=MenuItem::new(crate::i18n::text("Beenden"),true,None);
        menu.append_items(&[&show,&settings,&PredefinedMenuItem::separator(),&quit]).map_err(|error|error.to_string())?;
        let icon=Icon::from_rgba(icon_rgba(),32,32).map_err(|error|error.to_string())?;
        let tray=TrayIconBuilder::new().with_tooltip("ApexControl").with_icon(icon).with_menu(Box::new(menu)).with_menu_on_left_click(false).build().map_err(|error|error.to_string())?;
        let (sender,events)=mpsc::channel();
        let context=ctx.clone();let click_sender=sender.clone();
        TrayIconEvent::set_event_handler(Some(move |event| {
            if matches!(event,TrayIconEvent::Click{button:MouseButton::Left,button_state:MouseButtonState::Up,..}|TrayIconEvent::DoubleClick{button:MouseButton::Left,..}) {
                let _=click_sender.send(Action::Show);window.wake(&context);
            }
        }));
        let show_id=show.id().clone();let settings_id=settings.id().clone();let quit_id=quit.id().clone();let context=ctx.clone();
        MenuEvent::set_event_handler(Some(move |event:MenuEvent| {
            let action=if event.id==show_id {Action::Show}else if event.id==settings_id {Action::Settings}else if event.id==quit_id {Action::Quit}else{return};
            let _=sender.send(action);window.wake(&context);
        }));
        Ok(Self{_icon:tray,events,labels:[show,settings,quit]})
    }
    #[cfg(not(windows))]
    pub fn new(_:&eframe::CreationContext<'_>)->Result<Self,String> {Err(crate::i18n::text("Der Infobereich ist bisher nur unter Windows verfügbar").into())}
    pub fn events(&self)->impl Iterator<Item=Action>+'_ {self.events.try_iter()}
    pub fn update_language(&self) {
        #[cfg(windows)]
        for (item,label) in self.labels.iter().zip(["ApexControl öffnen","Einstellungen","Beenden"]) {item.set_text(crate::i18n::text(label));}
    }
}
pub fn show_window(ctx:&egui::Context) {
    ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
    ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
    ctx.send_viewport_cmd(egui::ViewportCommand::Focus);ctx.request_repaint();
}

pub fn icon_rgba()->Vec<u8> {
    crate::brand::icon_rgba()
}

#[derive(Debug,PartialEq)]
pub enum CloseAction {Wait,Hide,Exit}
pub fn close_action(settings:&Settings,tray_available:bool,busy:bool,explicit_quit:bool)->CloseAction {
    if busy {CloseAction::Wait}else if settings.minimize_on_close && tray_available && !explicit_quit {CloseAction::Hide}else{CloseAction::Exit}
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn first_run_is_completed_only_after_saved_language_and_layout(){
        let folder=std::env::temp_dir().join(format!("apexcontrol-first-run-test-{}",std::process::id()));
        let mut preferences=Settings::default();assert!(!preferences.language_setup_done);
        preferences.confirm_setup_in(crate::i18n::Language::English,crate::keyboard::Layout::SpanishIso,&folder).unwrap();
        let mut restored:Settings=serde_json::from_slice(&fs::read(folder.join("settings.json")).unwrap()).unwrap();
        assert!(restored.setup_complete());assert_eq!(restored.language,crate::i18n::Language::English);
        assert_eq!(restored.keyboard_layout,Some(crate::keyboard::Layout::SpanishIso));
        restored.language=crate::i18n::Language::German;restored.save_in(&folder).unwrap();
        let again:Settings=serde_json::from_slice(&fs::read(folder.join("settings.json")).unwrap()).unwrap();assert!(again.language_setup_done);
        let blocked=folder.join("blocked");fs::write(&blocked,b"file").unwrap();let mut fresh=Settings::default();
        assert!(fresh.confirm_setup_in(crate::i18n::Language::English,crate::keyboard::Layout::UsAnsi,&blocked).is_err());assert!(!fresh.setup_complete());assert!(fresh.keyboard_layout.is_none());
        fs::remove_file(blocked).unwrap();fs::remove_file(folder.join("settings.json")).unwrap();fs::remove_dir(folder).unwrap();
    }
    #[cfg(windows)]
    #[test]
    fn native_restore_makes_a_hidden_window_visible_without_a_paint_loop() {
        use windows_sys::Win32::{Foundation::HWND,UI::WindowsAndMessaging::*};
        struct TestWindow(HWND);
        impl Drop for TestWindow {fn drop(&mut self){unsafe{DestroyWindow(self.0);}}}
        let class:Vec<u16>="STATIC\0".encode_utf16().collect();
        // This is an owned, off-screen tool window, not the user's app window.
        // No event loop, repaint or tray interaction may be needed to restore it.
        let window=TestWindow(unsafe{CreateWindowExW(WS_EX_TOOLWINDOW|WS_EX_NOACTIVATE,class.as_ptr(),class.as_ptr(),WS_POPUP,
            -32000,-32000,1,1,std::ptr::null_mut(),std::ptr::null_mut(),std::ptr::null_mut(),std::ptr::null())});
        assert!(!window.0.is_null());let native=NativeWindow(window.0 as isize);
        unsafe {assert_eq!(IsWindowVisible(window.0),0);}
        assert!(native.restore(false));
        unsafe {ShowWindow(window.0,SW_HIDE);assert_eq!(IsWindowVisible(window.0),0);}
        assert!(native.restore(false));
        unsafe {ShowWindow(window.0,SW_SHOWMINNOACTIVE);assert_ne!(IsIconic(window.0),0);}
        assert!(native.restore(false));unsafe{assert_eq!(IsIconic(window.0),0);}
        drop(window);assert!(!native.restore(false));
    }
    #[test]
    fn close_never_hides_without_a_tray_or_interrupts_a_device_write() {
        let settings=Settings{minimize_on_close:true,autostart_minimized:true,..Default::default()};
        assert_eq!(close_action(&settings,true,false,false),CloseAction::Hide);
        assert_eq!(close_action(&settings,false,false,false),CloseAction::Exit);
        assert_eq!(close_action(&settings,true,false,true),CloseAction::Exit);
        assert_eq!(close_action(&settings,true,true,true),CloseAction::Wait);
    }
    #[test]
    fn autostart_quotes_paths_and_identifies_a_system_start() {
        assert_eq!(autostart_command(Path::new(r"C:\Apps With Spaces\ApexControl.exe")).unwrap(),r#""C:\Apps With Spaces\ApexControl.exe" --autostart"#);
        assert!(autostart_command(Path::new("bad\"path")).is_err());
    }
    #[test]
    fn preferences_roundtrip_preserves_defaults_and_overwrites_atomically() {
        let folder=std::env::temp_dir().join(format!("apexcontrol-settings-test-{}",std::process::id()));
        let first=Settings::default();first.save_in(&folder).unwrap();
        let second=Settings{minimize_on_close:true,autostart_minimized:true,language:crate::i18n::Language::English,..Default::default()};second.save_in(&folder).unwrap();
        let read:Settings=serde_json::from_slice(&fs::read(folder.join("settings.json")).unwrap()).unwrap();
        assert!(read==second);assert!(!folder.join("settings.tmp").exists());
        assert!(!serde_json::from_str::<Settings>("{}").unwrap().minimize_on_close);
        fs::remove_file(folder.join("settings.json")).unwrap();fs::remove_dir(folder).unwrap();
    }
    #[cfg(windows)]
    #[test]
    fn autostart_registry_roundtrip_only_changes_its_own_value() {
        // Exercise the real Windows registry API in a private temporary key,
        // never in the user's actual Windows Run key.
        let stamp=std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let path=format!(r"Software\ApexControl-Test-{}-{stamp}",std::process::id());
        let (root,_)=winreg::HKCU.create_subkey(&path).unwrap();
        let (key,_)=root.create_subkey("Run").unwrap();let (approved,_)=root.create_subkey("Approved").unwrap();
        key.set_value("Keep",&"untouched").unwrap();approved.set_value("Keep",&"untouched").unwrap();
        let command=autostart_command(Path::new(r"C:\Apps With Spaces\ApexControl.exe")).unwrap();
        assert!(!read_autostart(&key,Some(&approved),&command).unwrap());
        update_autostart(&key,&approved,true,&command).unwrap();assert_eq!(key.get_value::<String,_>("ApexControl").unwrap(),command);
        assert!(read_autostart(&key,Some(&approved),&command).unwrap());
        // Task Manager disables the approval entry without deleting Run.
        let mut disabled=vec![0;12];disabled[0]=3;disabled[11]=42;
        approved.set_raw_value("ApexControl",&winreg::RegValue{vtype:winreg::enums::REG_BINARY,bytes:disabled.into()}).unwrap();
        assert!(!read_autostart(&key,Some(&approved),&command).unwrap());
        assert!(read_autostart(&key,None,&command).unwrap());
        update_autostart(&key,&approved,true,&command).unwrap();assert!(read_autostart(&key,Some(&approved),&command).unwrap());
        update_autostart(&key,&approved,false,&command).unwrap();assert!(!read_autostart(&key,Some(&approved),&command).unwrap());
        assert_eq!(key.get_value::<String,_>("ApexControl").unwrap(),command,"disabled entry stays visible");
        let readonly=root.open_subkey("Approved").unwrap();
        assert!(update_autostart(&key,&readonly,true,"different command").is_err());
        assert_eq!(key.get_value::<String,_>("ApexControl").unwrap(),command,"failed approval update rolls back registration");
        assert_eq!(key.get_value::<String,_>("Keep").unwrap(),"untouched");assert_eq!(approved.get_value::<String,_>("Keep").unwrap(),"untouched");
        drop(readonly);drop(key);drop(approved);root.delete_subkey("Run").unwrap();root.delete_subkey("Approved").unwrap();
        drop(root);winreg::HKCU.delete_subkey(&path).unwrap();
    }
}
