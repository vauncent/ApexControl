//! Windows GUI instance ownership and a parameter-free "show window" signal.
#[cfg(windows)]
mod windows {
    use std::{io,ptr,sync::{Arc,atomic::{AtomicBool,Ordering}},thread};
    use eframe::egui;
    use windows_sys::Win32::{Foundation::{HANDLE,CloseHandle,GetLastError,ERROR_ALREADY_EXISTS,WAIT_OBJECT_0},
        Security::{GetTokenInformation,GetLengthSid,TokenUser,TOKEN_USER,TOKEN_QUERY},
        System::Threading::{CreateEventW,CreateMutexW,GetCurrentProcess,OpenProcessToken,SetEvent,WaitForSingleObject,INFINITE},
        UI::WindowsAndMessaging::{GetWindowThreadProcessId,IsWindow,ShowWindowAsync,SetForegroundWindow,SW_RESTORE}};

    struct Handle(HANDLE);
    // Kernel event/mutex/token handles support cross-thread operations. Arc
    // keeps each handle alive until the listener's final operation is complete.
    unsafe impl Send for Handle {}
    unsafe impl Sync for Handle {}
    impl Drop for Handle{fn drop(&mut self){unsafe{CloseHandle(self.0);}}}
    fn wide(value:&str)->Vec<u16>{value.encode_utf16().chain(Some(0)).collect()}
    fn user_key()->io::Result<String>{
        let mut token=ptr::null_mut();
        if unsafe{OpenProcessToken(GetCurrentProcess(),TOKEN_QUERY,&mut token)}==0{return Err(io::Error::last_os_error());}
        let token=Handle(token);let mut size=0;
        unsafe{GetTokenInformation(token.0,TokenUser,ptr::null_mut(),0,&mut size);}
        if size==0{return Err(io::Error::last_os_error());}
        // TOKEN_USER contains a pointer and must be properly aligned.
        let mut buffer=vec![0usize;(size as usize).div_ceil(std::mem::size_of::<usize>())];
        if unsafe{GetTokenInformation(token.0,TokenUser,buffer.as_mut_ptr().cast(),size,&mut size)}==0{return Err(io::Error::last_os_error());}
        let sid=unsafe{(*(buffer.as_ptr().cast::<TOKEN_USER>())).User.Sid};
        let bytes=unsafe{std::slice::from_raw_parts(sid.cast::<u8>(),GetLengthSid(sid) as usize)};
        Ok(bytes.iter().map(|b|format!("{b:02x}")).collect())
    }

    pub struct Guard {
        _mutex:Handle,event:Arc<Handle>,stop:Arc<AtomicBool>,show:Arc<AtomicBool>,listener:Option<thread::JoinHandle<()>>,
    }
    impl Guard {
        pub fn acquire(show_existing:bool)->io::Result<Option<Self>>{
            Self::named(&format!("Local\\ApexControl.GUI.v1.{}",user_key()?),show_existing)
        }
        fn named(name:&str,show_existing:bool)->io::Result<Option<Self>>{
            // Create the auto-reset event first. A second start can signal it
            // while the first instance is still constructing its window.
            let event=unsafe{CreateEventW(ptr::null(),0,0,wide(&format!("{name}.Show")).as_ptr())};
            if event.is_null(){return Err(io::Error::last_os_error());}let event=Arc::new(Handle(event));
            let mutex=unsafe{CreateMutexW(ptr::null(),0,wide(&format!("{name}.Lock")).as_ptr())};
            let existed=unsafe{GetLastError()}==ERROR_ALREADY_EXISTS;
            if mutex.is_null(){return Err(io::Error::last_os_error());}let mutex=Handle(mutex);
            if existed{
                if show_existing && unsafe{SetEvent(event.0)}==0{return Err(io::Error::last_os_error());}
                return Ok(None);
            }
            Ok(Some(Self{_mutex:mutex,event,stop:Arc::new(AtomicBool::new(false)),show:Arc::new(AtomicBool::new(false)),listener:None}))
        }
        pub fn take_show_request(&self)->bool{self.show.swap(false,Ordering::AcqRel)}
        pub fn attach(&mut self,cc:&eframe::CreationContext<'_>)->io::Result<()>{
            use raw_window_handle::{HasWindowHandle,RawWindowHandle};
            let window=match cc.window_handle().map_err(io::Error::other)?.as_raw(){
                RawWindowHandle::Win32(window)=>window.hwnd.get(),_=>return Err(io::Error::other("Missing Win32 window")),
            };
            self.listen(window,cc.egui_ctx.clone(),true)
        }
        fn listen(&mut self,window:isize,ctx:egui::Context,focus:bool)->io::Result<()>{
            let event=self.event.clone();let stop=self.stop.clone();let show=self.show.clone();
            self.listener=Some(thread::Builder::new().name("apexcontrol-instance".into()).spawn(move||{
                loop {
                    if unsafe{WaitForSingleObject(event.0,INFINITE)}!=WAIT_OBJECT_0 || stop.load(Ordering::Acquire){break;}
                    show.store(true,Ordering::Release);
                    let window=window as windows_sys::Win32::Foundation::HWND;
                    let mut owner=0;
                    unsafe{
                        if IsWindow(window)==0{break;}
                        GetWindowThreadProcessId(window,&mut owner);
                        if owner!=std::process::id(){break;}
                        // Post to the Win32 queue: waking egui alone cannot
                        // reliably restore a hidden window with no paint loop.
                        ShowWindowAsync(window,SW_RESTORE);
                        if focus{SetForegroundWindow(window);}
                    }
                    crate::desktop::show_window(&ctx);
                }
            })?);Ok(())
        }
    }
    impl Drop for Guard{
        fn drop(&mut self){
            self.stop.store(true,Ordering::Release);unsafe{SetEvent(self.event.0);}
            // Do not join on the UI thread during native window destruction.
            // The listener owns its event handle and exits after the wakeup.
        }
    }
    #[cfg(test)]
    mod tests {
        use super::*;
        fn name()->String{format!("Local\\ApexControl.Test.{}.{}",std::process::id(),std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos())}
        #[test]
        fn duplicate_launch_signals_existing_instance_and_autostart_stays_quiet(){
            let name=name();let primary=Guard::named(&name,true).unwrap().unwrap();
            assert!(Guard::named(&name,false).unwrap().is_none());
            assert_eq!(unsafe{WaitForSingleObject(primary.event.0,0)},windows_sys::Win32::Foundation::WAIT_TIMEOUT);
            assert!(Guard::named(&name,true).unwrap().is_none());
            assert_eq!(unsafe{WaitForSingleObject(primary.event.0,0)},WAIT_OBJECT_0);
            drop(primary);assert!(Guard::named(&name,true).unwrap().is_some());
        }
        #[test]
        fn simultaneous_launches_elect_exactly_one_primary(){
            let name=name();let barrier=Arc::new(std::sync::Barrier::new(12));
            let threads:Vec<_>=(0..12).map(|_|{let name=name.clone();let barrier=barrier.clone();thread::spawn(move||{
                barrier.wait();let guard=Guard::named(&name,false).unwrap();let first=guard.is_some();barrier.wait();drop(guard);first
            })}).collect();
            assert_eq!(threads.into_iter().map(|t|usize::from(t.join().unwrap())).sum::<usize>(),1);
        }
        #[test]
        fn own_user_namespace_is_stable(){assert_eq!(user_key().unwrap(),user_key().unwrap());assert!(!user_key().unwrap().is_empty());}
        #[test]
        fn pending_second_launch_restores_a_hidden_owned_window_without_painting(){
            use windows_sys::Win32::UI::WindowsAndMessaging::{CreateWindowExW,DestroyWindow,IsWindowVisible,PeekMessageW,TranslateMessage,DispatchMessageW,MSG,PM_REMOVE,WS_EX_TOOLWINDOW,WS_EX_NOACTIVATE,WS_POPUP};
            struct Window(windows_sys::Win32::Foundation::HWND);
            impl Drop for Window{fn drop(&mut self){unsafe{DestroyWindow(self.0);}}}
            let window=Window(unsafe{CreateWindowExW(WS_EX_TOOLWINDOW|WS_EX_NOACTIVATE,wide("STATIC").as_ptr(),wide("ApexControl instance test").as_ptr(),WS_POPUP,-32000,-32000,100,100,ptr::null_mut(),ptr::null_mut(),ptr::null_mut(),ptr::null())});
            assert!(!window.0.is_null());assert_eq!(unsafe{IsWindowVisible(window.0)},0);
            let name=name();let mut primary=Guard::named(&name,true).unwrap().unwrap();
            // Signal before GUI/listener setup, as during a double-click.
            assert!(Guard::named(&name,true).unwrap().is_none());
            primary.listen(window.0 as isize,egui::Context::default(),false).unwrap();
            let deadline=std::time::Instant::now()+std::time::Duration::from_secs(2);
            while unsafe{IsWindowVisible(window.0)}==0 && std::time::Instant::now()<deadline{
                let mut message:MSG=unsafe{std::mem::zeroed()};
                while unsafe{PeekMessageW(&mut message,window.0,0,0,PM_REMOVE)}!=0{unsafe{TranslateMessage(&message);DispatchMessageW(&message);}}
                thread::sleep(std::time::Duration::from_millis(1));
            }
            assert_ne!(unsafe{IsWindowVisible(window.0)},0);assert!(primary.take_show_request());assert!(!primary.take_show_request());
            drop(primary);
        }
    }
}
#[cfg(windows)] pub use windows::Guard;

#[cfg(not(windows))]
pub struct Guard;
#[cfg(not(windows))]
impl Guard{
    pub fn acquire(_:bool)->std::io::Result<Option<Self>>{Ok(Some(Self))}
    pub fn attach(&mut self,_:&eframe::CreationContext<'_>)->std::io::Result<()>{Ok(())}
    pub fn take_show_request(&self)->bool{false}
}
