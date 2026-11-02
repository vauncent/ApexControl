//! Transient lighting triggers; no text conversion, recording, file or network IO.
use std::{collections::VecDeque,sync::{Arc,Mutex,atomic::{AtomicBool,Ordering}}};
#[derive(Default)]struct State{active:AtomicBool,keys:Mutex<VecDeque<u8>>,#[cfg(windows)] registration:Mutex<Option<(isize,u32)>>}
#[derive(Clone,Default)]pub struct Input(Arc<State>);
impl Input{
    pub fn drain(&self)->Vec<u8>{self.0.keys.lock().map(|mut keys|keys.drain(..).collect()).unwrap_or_default()}
    pub fn disable(&self){
        self.0.active.store(false,Ordering::Release);self.drain();
        #[cfg(windows)] if let Ok(mut previous)=self.0.registration.lock(){if let Some((window,flags))=previous.take(){
            use windows_sys::Win32::UI::Input::{RegisterRawInputDevices,RAWINPUTDEVICE};
            let device=RAWINPUTDEVICE{usUsagePage:1,usUsage:6,dwFlags:flags,hwndTarget:window as _};
            unsafe{RegisterRawInputDevices(&device,1,std::mem::size_of::<RAWINPUTDEVICE>() as u32);}
        }}
    }
    #[cfg(not(windows))]pub fn enable(&self)->Result<(),String>{Err(crate::i18n::text("Reaktive Beleuchtung ist bisher nur unter Windows verfügbar").into())}
    #[cfg(windows)]pub fn enable(&self)->Result<(),String>{
        use windows_sys::Win32::UI::{Input::*,WindowsAndMessaging::{GetWindowThreadProcessId,IsWindow}};
        self.disable();let mut count=0;let size=std::mem::size_of::<RAWINPUTDEVICE>() as u32;
        if unsafe{GetRegisteredRawInputDevices(std::ptr::null_mut(),&mut count,size)}==u32::MAX||count==0||count>64{return Err("Windows raw input registration unavailable".into());}
        let mut devices=vec![unsafe{std::mem::zeroed::<RAWINPUTDEVICE>()};count as usize];
        let count=unsafe{GetRegisteredRawInputDevices(devices.as_mut_ptr(),&mut count,size)};
        if count==u32::MAX||count as usize>devices.len(){return Err(std::io::Error::last_os_error().to_string());}
        let original=devices[..count as usize].iter().find(|d|d.usUsagePage==1&&d.usUsage==6).ok_or("No keyboard raw input target")?;
        let mut owner=0;
        unsafe{GetWindowThreadProcessId(original.hwndTarget,&mut owner);}
        if original.hwndTarget.is_null()||unsafe{IsWindow(original.hwndTarget)}==0||owner!=std::process::id(){return Err("Invalid raw input target".into());}
        // Retain winit's own target window. Registering another window would
        // steal the process's keyboard registration and could break GUI input.
        // Winit registers DEVNOTIFY. The enumeration on Windows does not
        // reliably round-trip that bit, so retain our application's setting.
        let original_flags=original.dwFlags|RIDEV_DEVNOTIFY;
        let device=RAWINPUTDEVICE{usUsagePage:1,usUsage:6,dwFlags:(original_flags|RIDEV_INPUTSINK)&!RIDEV_EXINPUTSINK,hwndTarget:original.hwndTarget};
        if unsafe{RegisterRawInputDevices(&device,1,size)}==0{return Err(std::io::Error::last_os_error().to_string());}
        *self.0.registration.lock().map_err(|_|"Raw input lock failed")?=Some((original.hwndTarget as isize,original_flags));
        self.0.active.store(true,Ordering::Release);Ok(())
    }
    #[cfg(windows)]fn message(&self,message:*const std::ffi::c_void){
        use windows_sys::Win32::UI::{Input::*,WindowsAndMessaging::{MSG,WM_INPUT}};
        if !self.0.active.load(Ordering::Acquire)||message.is_null(){return;}
        // The pointer is supplied by winit's native message hook and is valid
        // for this callback only. The message is always passed on to winit.
        let message=unsafe{&*message.cast::<MSG>()};if message.message!=WM_INPUT{return;}
        let mut raw:RAWINPUT=unsafe{std::mem::zeroed()};let mut size=std::mem::size_of::<RAWINPUT>() as u32;
        let length=unsafe{GetRawInputData(message.lParam as _,RID_INPUT,(&mut raw as *mut RAWINPUT).cast(),&mut size,std::mem::size_of::<RAWINPUTHEADER>() as u32)};
        if length==u32::MAX||length<(std::mem::size_of::<RAWINPUTHEADER>()+std::mem::size_of::<RAWKEYBOARD>()) as u32||length>std::mem::size_of::<RAWINPUT>() as u32||raw.header.dwType!=RIM_TYPEKEYBOARD{return;}
        let key=unsafe{raw.data.keyboard};if key.Flags&1!=0||key.Flags&4!=0{return;}
        // Query only enough device identity to filter the supported Mini. The
        // raw device name is not retained, and unrelated keyboards are ignored.
        let mut count=0;
        if unsafe{GetRawInputDeviceInfoW(raw.header.hDevice,RIDI_DEVICENAME,std::ptr::null_mut(),&mut count)}==u32::MAX||count==0||count>1024{return;}
        let mut name=vec![0u16;count as usize+1];let capacity=name.len() as u32;
        count=capacity;
        if unsafe{GetRawInputDeviceInfoW(raw.header.hDevice,RIDI_DEVICENAME,name.as_mut_ptr().cast(),&mut count)}==u32::MAX||count>capacity{return;}
        if !mini_device(&String::from_utf16_lossy(&name[..count as usize])){return;}
        if let Some(hid)=scan_hid(key.MakeCode,key.Flags&2!=0){if let Ok(mut keys)=self.0.keys.lock(){if keys.len()==64{keys.pop_front();}keys.push_back(hid);}}
    }
}
#[cfg(windows)]pub fn install(options:&mut eframe::NativeOptions,input:Input){
    use winit::platform::windows::EventLoopBuilderExtWindows;
    options.event_loop_builder=Some(Box::new(move|builder|{builder.with_msg_hook(move|message|{input.message(message);false});}));
}
#[cfg(not(windows))]pub fn install(_:&mut eframe::NativeOptions,_:Input){}
#[cfg(any(windows,test))]fn mini_device(name:&str)->bool{let upper=name.to_ascii_uppercase();let tokens:Vec<_>=upper.split(['#','&','\\','\0']).collect();tokens.contains(&"VID_1038")&&tokens.contains(&"PID_161E")}
#[cfg(any(windows,test))]fn scan_hid(scan:u16,extended:bool)->Option<u8>{
    if extended{return match scan{0x1c=>Some(88),0x1d=>Some(228),0x38=>Some(230),0x5b=>Some(227),0x5c=>Some(231),_=>None};}
    Some(match scan{
        0x01=>41,0x02..=0x0b=>(scan+28) as u8,0x0c=>45,0x0d=>46,0x0e=>42,0x0f=>43,
        0x10=>20,0x11=>26,0x12=>8,0x13=>21,0x14=>23,0x15=>28,0x16=>24,0x17=>12,0x18=>18,0x19=>19,0x1a=>47,0x1b=>48,0x1c=>40,0x1d=>224,
        0x1e=>4,0x1f=>22,0x20=>7,0x21=>9,0x22=>10,0x23=>11,0x24=>13,0x25=>14,0x26=>15,0x27=>51,0x28=>52,0x29=>53,0x2a=>225,0x2b=>49,
        0x2c=>29,0x2d=>27,0x2e=>6,0x2f=>25,0x30=>5,0x31=>17,0x32=>16,0x33=>54,0x34=>55,0x35=>56,0x36=>229,0x38=>226,0x39=>44,0x3a=>57,0x56=>100,
        0x70=>136,0x73=>135,0x79=>138,0x7b=>139,0x7d=>137,_=>return None,
    })
}
#[cfg(test)]mod tests{
    use super::*;
    #[test]fn device_filter_and_physical_scan_mapping_do_not_depend_on_input_language(){assert!(mini_device(r"\\?\HID#VID_1038&PID_161E&MI_00"));assert!(!mini_device("VID_1038&PID_1614"));assert!(!mini_device("VID_1234&PID_161E"));assert!(!mini_device("VID_1038&PID_161E0"));assert_eq!(scan_hid(0x30,false),Some(5));assert_eq!(scan_hid(0x1d,true),Some(228));assert_eq!(scan_hid(0x7d,false),Some(137));}
    #[cfg(windows)]
    #[test]fn background_input_preserves_the_existing_target_and_restores_its_flags(){
        use windows_sys::Win32::UI::{Input::*,WindowsAndMessaging::*};
        let class:Vec<u16>="STATIC\0".encode_utf16().collect();let title:Vec<u16>="ApexControl RGB input test\0".encode_utf16().collect();
        let window=unsafe{CreateWindowExW(WS_EX_TOOLWINDOW|WS_EX_NOACTIVATE,class.as_ptr(),title.as_ptr(),WS_POPUP,-32000,-32000,100,100,std::ptr::null_mut(),std::ptr::null_mut(),std::ptr::null_mut(),std::ptr::null())};assert!(!window.is_null());
        struct Cleanup(windows_sys::Win32::Foundation::HWND);
        impl Drop for Cleanup{fn drop(&mut self){let remove=RAWINPUTDEVICE{usUsagePage:1,usUsage:6,dwFlags:RIDEV_REMOVE,hwndTarget:std::ptr::null_mut()};unsafe{RegisterRawInputDevices(&remove,1,std::mem::size_of::<RAWINPUTDEVICE>() as u32);DestroyWindow(self.0);}}}
        let _cleanup=Cleanup(window);
        let original=RAWINPUTDEVICE{usUsagePage:1,usUsage:6,dwFlags:RIDEV_DEVNOTIFY,hwndTarget:window};assert_ne!(unsafe{RegisterRawInputDevices(&original,1,std::mem::size_of::<RAWINPUTDEVICE>() as u32)},0);
        let input=Input::default();input.enable().unwrap();assert_eq!(*input.0.registration.lock().unwrap(),Some((window as isize,RIDEV_DEVNOTIFY)));
        let mut count=8;let mut active=vec![unsafe{std::mem::zeroed::<RAWINPUTDEVICE>()};8];let found=unsafe{GetRegisteredRawInputDevices(active.as_mut_ptr(),&mut count,std::mem::size_of::<RAWINPUTDEVICE>() as u32)};assert_ne!(found,u32::MAX);
        let active=active[..found as usize].iter().find(|entry|entry.usUsagePage==1&&entry.usUsage==6).unwrap();assert_eq!(active.hwndTarget,window);assert_ne!(active.dwFlags&RIDEV_INPUTSINK,0);
        input.disable();assert!(!input.0.active.load(Ordering::Acquire));
        let mut count=8;let mut registrations=vec![unsafe{std::mem::zeroed::<RAWINPUTDEVICE>()};8];
        let found=unsafe{GetRegisteredRawInputDevices(registrations.as_mut_ptr(),&mut count,std::mem::size_of::<RAWINPUTDEVICE>() as u32)};assert_ne!(found,u32::MAX);
        let restored=registrations[..found as usize].iter().find(|entry|entry.usUsagePage==1&&entry.usUsage==6).unwrap();assert_eq!(restored.hwndTarget,window);assert_eq!(restored.dwFlags&!RIDEV_DEVNOTIFY,0);
    }
}
