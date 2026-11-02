use eframe::egui::{self,Color32,FontId,RichText,Stroke,Vec2,Pos2,Rect,Sense,Align2};
use crate::{backend::{self,Device,SavedProfile,Request,Response},protocol::{self,Profile,MacroEvent,MacroRecord,KeyCategory},desktop::{self,Settings,Desktop,Action,CloseAction}};
use std::{collections::{BTreeSet,BTreeMap},sync::mpsc,time::{Duration,Instant},fs};

const BG:Color32=Color32::from_rgb(16,18,23);
const PANEL:Color32=Color32::from_rgb(24,27,34);
const CARD:Color32=Color32::from_rgb(29,33,41);
const LINE:Color32=Color32::from_rgb(48,54,65);
const TEXT:Color32=Color32::from_rgb(234,237,242);
const MUTED:Color32=Color32::from_rgb(142,151,168);
const ORANGE:Color32=Color32::from_rgb(255,143,79);
const GREEN:Color32=Color32::from_rgb(105,213,166);
const BLUE:Color32=Color32::from_rgb(134,166,255);
const RELEASES_URL:&str="https://github.com/vincentmittermaier-jpg/ApexControl/releases";

#[derive(Clone,Copy,PartialEq)] enum Tab {Actuation,Bindings,Macros,Rgb}
struct BindingPicker {keys:Vec<u8>,bindings:Vec<[u8;5]>,category:KeyCategory,target:u8}
impl BindingPicker {
    fn for_selection(profile:&Profile,keys:Vec<u8>)->Self {
        let bindings:Vec<_>=keys.iter().filter_map(|key|profile.value(*key).map(|state|state.binding)).collect();
        let target=bindings.first().filter(|binding|binding[0]==0x51).map_or(keys[0],|binding|binding[1]);
        let category=KeyCategory::ALL.into_iter().find(|category|category.keys().contains(&target)).unwrap_or(KeyCategory::Letters);
        Self{keys,bindings,category,target}
    }
    fn matches(&self,profile:&Profile,keys:&[u8])->bool {
        self.keys==keys && self.bindings==keys.iter().filter_map(|key|profile.value(*key).map(|state|state.binding)).collect::<Vec<_>>()
    }
}
#[derive(Clone,serde::Serialize,serde::Deserialize)]
struct Session {device_id:String,slot:u8,draft:Option<Profile>,name:String,#[serde(default)] checkpoint:Option<Draft>}
#[derive(Clone,PartialEq,serde::Serialize,serde::Deserialize)]
struct Draft {profile:Profile,name:String}
enum PendingAction {Open(usize),Load(usize,u8),Local(usize,usize),Scan,Home,Quit,Reset,Backup(usize,u8,Profile)}
#[derive(Default)]
struct EditorSafety {checkpoint:Option<Draft>,pending:Option<PendingAction>}
#[derive(Default)]
struct BackupBrowser {open:bool,entries:Vec<backend::Backup>,rejected:usize,error:Option<String>,choice:Option<usize>,device:Option<usize>,slot:u8}
struct RgbUi{mode:crate::rgb::Mode,speed:f32,colors:BTreeMap<String,BTreeMap<u8,[u8;3]>>,preview:crate::rgb::Preview,active:Option<String>,pending_device:Option<String>,pending_frame:Option<crate::rgb::Frame>}
impl Default for RgbUi{fn default()->Self{Self{mode:crate::rgb::Mode::Static,speed:1.0,colors:BTreeMap::new(),preview:Default::default(),active:None,pending_device:None,pending_frame:None}}}
#[derive(Default)]struct KeyboardInteraction{marquee:Option<Marquee>}
struct Marquee{start:Pos2,end:Pos2,initial:BTreeSet<u8>,additive:bool,dragged:bool}
impl Session {
    fn read()->Option<Self> {
        let session:Self=serde_json::from_slice(&crate::storage::read(&backend::data_dir().join("session.json"),crate::storage::PROFILE_LIMIT).ok()?).ok()?;
        if session.slot>4 || session.draft.as_ref().is_some_and(|p|Profile::parse(p.bytes.clone()).is_err()) {return None;}
        Some(session)
    }
}
pub struct ApexApp {
    _instance:Option<crate::instance::Guard>,
    safety:EditorSafety,backups:BackupBrowser,
    rgb_ui:RgbUi,keyboard_interaction:KeyboardInteraction,
    tx:mpsc::Sender<Request>,rx:mpsc::Receiver<Response>,devices:Vec<Device>,saved:Vec<SavedProfile>,
    device:Option<usize>,editor:bool,busy:bool,status:String,error:bool,
    slot:u8,profile:Option<Profile>,baseline:Option<Profile>,selected:BTreeSet<u8>,tab:Tab,
    profile_name:String,live_preview:bool,card_choices:Vec<usize>,recording:bool,
    recorded:Vec<MacroEvent>,held:BTreeSet<u8>,last_event:Option<Instant>,macro_name:String,
    library:Vec<MacroRecord>,macro_choice:usize,
    rgb_color:[u8;3],rgb_brightness:u8,
    session:Option<Session>,
    macro_category:KeyCategory,macro_key:u8,macro_action:u8,macro_delay:u16,
    binding_picker:Option<BindingPicker>,
    settings:Settings,settings_open:bool,autostart:bool,autostart_error:Option<String>,
    autostart_poll:Option<Instant>,
    updates_open:bool,scan_failed:bool,
    desktop:Option<Desktop>,desktop_error:Option<String>,start_hidden:Option<u64>,quit_pending:bool,allow_exit:bool,
    intro_language:Option<crate::i18n::Language>,intro_layout:Option<crate::keyboard::Layout>,intro_focus_set:bool,intro_error:Option<String>,
}

fn frame() -> egui::Frame {egui::Frame::new().fill(PANEL).stroke(Stroke::new(1.0_f32,LINE)).corner_radius(12).inner_margin(18)}
fn card<R>(ui:&mut egui::Ui,contents:impl FnOnce(&mut egui::Ui)->R) -> egui::InnerResponse<egui::InnerResponse<R>> {
    frame().show(ui,|ui|ui.with_layout(egui::Layout::top_down(egui::Align::Min),contents))
}
fn primary(text:&str) -> egui::Button<'_> {egui::Button::new(RichText::new(text).color(BG).strong()).wrap_mode(egui::TextWrapMode::Extend).fill(ORANGE).corner_radius(7).min_size(Vec2::new(140.0,36.0))}
fn secondary(text:&str) -> egui::Button<'_> {egui::Button::new(RichText::new(text).color(TEXT)).wrap_mode(egui::TextWrapMode::Extend).fill(CARD).stroke(Stroke::new(1.0_f32,LINE)).corner_radius(7).min_size(Vec2::new(110.0,34.0))}
fn small(ui:&mut egui::Ui,text:&str) {ui.label(RichText::new(text).size(12.0).color(MUTED));}
fn pill(ui:&mut egui::Ui,text:&str,color:Color32) {egui::Frame::new().fill(color.gamma_multiply(0.12)).corner_radius(5).inner_margin(egui::Margin::symmetric(8,4)).show(ui,|ui|{ui.label(RichText::new(text).size(11.0).strong().color(color));});}
fn gear_button(ui:&mut egui::Ui)->egui::Response {
    let (rect,response)=ui.allocate_exact_size(Vec2::splat(36.0),Sense::click());let painter=ui.painter();let center=rect.center();
    painter.rect_filled(rect,7,if response.hovered(){CARD}else{PANEL});
    for tooth in 0..8 {let angle=tooth as f32*std::f32::consts::TAU/8.0;let direction=Vec2::new(angle.cos(),angle.sin());
        painter.line_segment([center+direction*8.0,center+direction*12.0],Stroke::new(3.0_f32,MUTED));}
    painter.circle_stroke(center,8.0,Stroke::new(2.5_f32,MUTED));painter.circle_stroke(center,3.0,Stroke::new(2.0_f32,MUTED));
    response.widget_info(||egui::WidgetInfo::labeled(egui::WidgetType::Button,ui.is_enabled(),crate::i18n::text("Einstellungen")));response.on_hover_text(crate::i18n::text("Einstellungen"))
}

impl ApexApp {
    pub fn new(cc:&eframe::CreationContext<'_>,instance:crate::instance::Guard,input:crate::rgb_input::Input)->Self {
        let mut style=egui::Style::default();style.visuals=egui::Visuals::dark();
        style.visuals.panel_fill=BG;style.visuals.window_fill=PANEL;style.visuals.extreme_bg_color=BG;
        style.visuals.override_text_color=Some(TEXT);style.visuals.selection.bg_fill=ORANGE.gamma_multiply(0.25);style.visuals.selection.stroke=Stroke::new(1.0_f32,ORANGE);
        style.visuals.widgets.inactive.bg_fill=CARD;style.visuals.widgets.inactive.weak_bg_fill=CARD;
        style.visuals.widgets.inactive.bg_stroke=Stroke::new(1.0_f32,LINE);style.visuals.widgets.hovered.bg_fill=Color32::from_rgb(44,49,59);
        style.visuals.widgets.active.bg_fill=ORANGE.gamma_multiply(0.4);style.visuals.widgets.active.fg_stroke=Stroke::new(1.0_f32,TEXT);
        style.spacing.item_spacing=Vec2::new(10.0,10.0);style.spacing.button_padding=Vec2::new(13.0,8.0);
        style.text_styles.insert(egui::TextStyle::Body,FontId::proportional(14.0));style.text_styles.insert(egui::TextStyle::Button,FontId::proportional(14.0));
        cc.egui_ctx.set_style(style);
        if let Ok(bytes)=fs::read("C:/Windows/Fonts/segoeui.ttf") {
            let mut fonts=egui::FontDefinitions::default();fonts.font_data.insert("segoe".into(),egui::FontData::from_owned(bytes).into());fonts.families.entry(egui::FontFamily::Proportional).or_default().insert(0,"segoe".into());cc.egui_ctx.set_fonts(fonts);
        }
        let settings=Settings::load();crate::i18n::set_language(if settings.language_setup_done{settings.language}else{crate::i18n::Language::English});
        let (tx,rx)=backend::worker(input);let _=tx.send(Request::Scan);
        let library=backend::load_macros();
        let (desktop,desktop_error)=match Desktop::new(cc){Ok(tray)=>(Some(tray),None),Err(error)=>(None,Some(error))};
        let (autostart,autostart_error)=match desktop::autostart_enabled(){Ok(enabled)=>(enabled,None),Err(error)=>(false,Some(error))};
        // eframe reveals its root window after the first painted frame. Hide only
        // on the following frame so that its initial reveal cannot undo this.
        let start_hidden=if settings.setup_complete() && settings.autostart_minimized && desktop.is_some() && std::env::args().any(|argument|argument=="--autostart") {
            Some(cc.egui_ctx.cumulative_frame_nr()+1)
        }else{None};
        let intro_language=if settings.language_setup_done && !settings.layout_setup_done{Some(settings.language)}else{None};
        Self {_instance:Some(instance),safety:EditorSafety::default(),backups:BackupBrowser::default(),rgb_ui:RgbUi::default(),keyboard_interaction:KeyboardInteraction::default(),tx,rx,devices:Vec::new(),saved:backend::saved_profiles(),device:None,editor:false,busy:true,status:crate::i18n::text("Verbundene Tastaturen werden gesucht …").into(),error:false,
            slot:2,profile:None,baseline:None,selected:BTreeSet::from([5]),tab:Tab::Actuation,profile_name:String::new(),live_preview:false,card_choices:Vec::new(),
            recording:false,recorded:Vec::new(),held:BTreeSet::new(),last_event:None,macro_name:crate::i18n::text("Mein Makro").into(),library,macro_choice:0,rgb_color:[255,143,79],rgb_brightness:65,
            session:Session::read(),macro_category:KeyCategory::Function,macro_key:58,macro_action:0,macro_delay:100,binding_picker:None,
            settings,settings_open:false,updates_open:false,scan_failed:false,autostart,autostart_error,autostart_poll:Some(Instant::now()),desktop,desktop_error,start_hidden,quit_pending:false,allow_exit:false,
            intro_language,intro_layout:None,intro_focus_set:false,intro_error:None}
    }
    fn tell(&mut self,message:impl Into<String>,error:bool) {self.status=message.into();self.error=error;}
    fn draft(&self)->Option<Draft>{self.profile.clone().map(|profile|Draft{profile,name:self.profile_name.clone()})}
    fn dirty(&self)->bool{self.profile.is_some() && self.draft()!=self.safety.checkpoint}
    fn mark_clean(&mut self){self.safety.checkpoint=self.draft();}
    fn reset_draft(&mut self){
        if let Some(draft)=self.safety.checkpoint.clone(){self.profile=Some(draft.profile);self.profile_name=draft.name;}
        self.live_preview=false;self.binding_picker=None;self.remember(true);
    }
    fn request_action(&mut self,action:PendingAction){
        if matches!(action,PendingAction::Quit){self.stop_recording();}
        if self.busy || self.recording || self.safety.pending.is_some(){return;}
        if self.dirty(){self.safety.pending=Some(action);}else{self.execute_action(action);}
    }
    fn execute_action(&mut self,action:PendingAction){
        match action{
            PendingAction::Open(index)=>self.open_device_now(index),
            PendingAction::Load(index,slot)=>self.load_slot_now(index,slot),
            PendingAction::Local(index,saved)=>self.local_draft_now(index,saved),
            PendingAction::Scan=>self.command(Request::Scan),
            PendingAction::Home=>self.editor=false,
            PendingAction::Quit=>{self.allow_exit=true;self.quit_pending=true;},
            PendingAction::Reset=>{self.reset_draft();self.tell(crate::i18n::text("Entwurf zurückgesetzt. Bereits live übertragene Einstellungen bleiben bestehen, bis du erneut live anwendest oder ein Onboard-Profil lädst."),false);},
            PendingAction::Backup(index,slot,profile)=>{
                let Some(device)=self.devices.get(index).filter(|device|device.supported && device.error.is_none()) else{return;};
                let Some(baseline)=device.profiles.get(slot as usize).cloned() else{return;};
                self.safety.checkpoint=Some(Draft{profile:baseline.clone(),name:baseline.name()});
                self.device=Some(index);self.slot=slot;self.baseline=Some(baseline);self.profile_name=profile.name();self.profile=Some(profile);
                self.live_preview=false;self.binding_picker=None;self.editor=true;self.backups.open=false;self.remember(true);
                self.tell(crate::i18n::text("Sicherung als Entwurf geöffnet. Prüfe das Profil und klicke zum Wiederherstellen auf „Auf Tastatur speichern“."),false);
            }
        }
    }
    fn resolve_pending(&mut self,save:bool){
        if self.safety.pending.is_none(){return;}
        if save && !self.save_local(){return;}
        let Some(action)=self.safety.pending.take() else{return;};
        if !save && !matches!(action,PendingAction::Reset){self.reset_draft();}self.execute_action(action);
    }
    fn command(&mut self,request:Request) {if self.tx.send(request).is_ok() {self.busy=true;self.error=false;self.status=crate::i18n::text("Einen Moment …").into();}}
    fn remember(&mut self,draft:bool) {
        let Some(index)=self.device else{return};
        let session=Session{device_id:self.devices[index].id.clone(),slot:self.slot,draft:if draft{self.profile.clone()}else{None},name:self.profile_name.clone(),checkpoint:if draft{self.safety.checkpoint.clone()}else{None}};
        // Headless UI fixtures keep their session in memory; filesystem behavior
        // is exercised by storage tests, never via the user's actual profiles.
        if cfg!(test){self.session=Some(session);return;}
        let save=(||{crate::storage::write(&backend::data_dir().join("session.json"),&serde_json::to_vec(&session)?,crate::storage::PROFILE_LIMIT)})()
            .map_err(|error:std::io::Error|error.to_string());
        self.session=Some(session);
        if let Err(error)=save {self.tell(crate::i18n::format_text("Profilansicht konnte nicht für den nächsten Start gemerkt werden: {error}", &[format!("{}", error)]),true);}
    }
    fn open_device(&mut self,index:usize) {
        self.request_action(PendingAction::Open(index));
    }
    fn open_device_now(&mut self,index:usize) {
        self.binding_picker=None;
        self.device=Some(index);self.editor=true;self.profile=None;self.baseline=None;
        let device=&self.devices[index];
        let remembered=self.session.as_ref().filter(|s|s.device_id==device.id);
        self.slot=remembered.map_or(0,|s|s.slot);
        if let Some(baseline)=device.profiles.get(self.slot as usize).cloned() {
            let profile=remembered.and_then(|s|s.draft.clone()).unwrap_or_else(||baseline.clone());
            self.profile_name=remembered.filter(|s|s.draft.is_some()).map_or_else(||profile.name(),|s|s.name.clone());
            self.safety.checkpoint=remembered.and_then(|session|session.checkpoint.clone()).or_else(||Some(Draft{profile:baseline.clone(),name:baseline.name()}));
            self.baseline=Some(baseline);self.profile=Some(profile);
            self.tell(if remembered.is_some(){crate::i18n::text("Zuletzt verwendetes Profil geöffnet. Der aktive Onboard-Slot wird nicht abgefragt.")}else{crate::i18n::text("Profilansicht bereit. Der aktive Onboard-Slot ist unbekannt; beim Start wurde kein Profil umgeschaltet.")},false);
        }
    }
    fn poll(&mut self) {
        while let Ok(response)=self.rx.try_recv() {
            if !matches!(response,Response::RgbError(_)){self.busy=false;}
            match response {
                Response::Scanned(result)=>match result {Ok(devices)=>{
                    self.scan_failed=false;
                    self.devices=devices;self.device=None;self.editor=false;self.profile=None;self.baseline=None;self.binding_picker=None;self.safety.checkpoint=None;
                    self.card_choices=self.devices.iter().map(|d|self.session.as_ref().filter(|s|s.device_id==d.id).map_or(0,|s|s.slot as usize)).collect();
                    self.tell(if self.devices.is_empty(){crate::i18n::text("Keine Apex-Tastatur gefunden. Bitte per USB verbinden.")}else{crate::i18n::text("Tastaturen aktualisiert.")},false);
                },Err(e)=>{self.scan_failed=true;self.tell(e,true)}},
                Response::Loaded(result)=>match result {Ok((slot,profile))=>{if let Some(index)=self.device {if let Some(cached)=self.devices[index].profiles.get_mut(slot as usize) {*cached=profile.clone();}}self.slot=slot;self.profile_name=profile.name();self.baseline=Some(profile.clone());self.profile=Some(profile);self.mark_clean();self.editor=true;self.tell(crate::i18n::format_text("Onboard-Profil {} geladen.", &[format!("{}", slot+1)]),false);self.remember(false);},Err(e)=>self.tell(e,true)},
                Response::Applied(result)=>match result {Ok(())=>{self.tell(crate::i18n::text("Auslösepunkt, Rapid Trigger und Schutzmodus live übernommen."),false);self.remember(true);},Err(e)=>self.tell(e,true)},
                Response::Rgb(result)=>{
                    self.rgb_ui.active=None;if let Ok(mut preview)=self.rgb_ui.preview.lock(){preview.take();}
                    match result {Ok(())=>{if let (Some(id),Some(frame))=(self.rgb_ui.pending_device.take(),self.rgb_ui.pending_frame.take()){self.rgb_ui.colors.entry(id).or_default().extend(frame);}self.tell(crate::i18n::text("RGB-Farbe live übernommen. Onboard-Profile bleiben unverändert."),false);},Err(e)=>self.tell(e,true)}
                },
                Response::RgbStarted(result)=>{self.rgb_ui.active=None;match result{Ok(())=>{self.rgb_ui.active=self.rgb_ui.pending_device.take();self.tell(crate::i18n::text("RGB-Effekt läuft. ApexControl muss dafür geöffnet bleiben, auch im Infobereich."),false);},Err(error)=>self.tell(error,true)}},
                Response::RgbError(error)=>{self.rgb_ui.active=None;self.tell(error,true);},
                Response::ClearedRgb(result)=>{
                    self.rgb_ui.active=None;if let Ok(mut preview)=self.rgb_ui.preview.lock(){preview.take();}
                    match result {Ok(())=>{if let Some(id)=self.rgb_ui.pending_device.take(){self.rgb_ui.colors.remove(&id);}self.tell(crate::i18n::text("Die Beleuchtung wurde wieder an das Onboard-Profil übergeben."),false);},Err(e)=>self.tell(e,true)}
                },
                Response::Stored(result)=>match result {Ok((slot,profile))=>{if let Some(index)=self.device {if let Some(p)=self.devices[index].profiles.get_mut(slot as usize) {*p=profile.clone();}}
                    self.profile_name=profile.name();self.baseline=Some(profile.clone());self.profile=Some(profile);self.mark_clean();self.tell(crate::i18n::text("Auf der Tastatur gespeichert und erfolgreich zurückgelesen."),false);self.remember(false);},Err(e)=>self.tell(e,true)},
            }
        }
        let preview=self.rgb_ui.preview.lock().ok().and_then(|mut latest|latest.take());
        if let Some((id,frame))=preview{self.rgb_ui.colors.entry(id).or_default().extend(frame);}
    }
    fn load_slot(&mut self,index:usize,slot:u8) {
        self.request_action(PendingAction::Load(index,slot));
    }
    fn load_slot_now(&mut self,index:usize,slot:u8) {
        if self.busy {return;}self.device=Some(index);let d=&self.devices[index];
        self.profile=None;self.baseline=None;self.safety.checkpoint=None;self.binding_picker=None;self.editor=true;
        if !d.supported {self.editor=true;return;}
        self.command(Request::Load{path:d.path.clone(),slot});
    }
    fn local_draft(&mut self,index:usize,saved_index:usize) {
        self.request_action(PendingAction::Local(index,saved_index));
    }
    fn local_draft_now(&mut self,index:usize,saved_index:usize) {
        self.binding_picker=None;
        if self.busy {return;}let Some(saved)=self.saved.get(saved_index).cloned() else{return};
        self.device=Some(index);self.profile_name=saved.name;self.profile=Some(saved.profile);
        self.baseline=self.devices[index].profiles.get(self.slot as usize).cloned();self.editor=true;
        self.mark_clean();
        self.tell(crate::i18n::text("Lokales Profil geöffnet. Zum Übernehmen live anwenden oder auf der Tastatur speichern."),false);
        self.remember(true);
    }
    fn apply(&mut self) {
        if self.busy {return;}if let (Some(index),Some(profile))=(self.device,self.profile.clone()) {self.command(Request::Apply{path:self.devices[index].path.clone(),profile});}
    }
    fn store(&mut self) {
        if self.busy {return;}
        if let (Some(index),Some(mut profile),Some(expected))=(self.device,self.profile.clone(),self.baseline.clone()) {
            if self.profile_name!=profile.name(){profile.set_name(&self.profile_name);}
            self.command(Request::Store{path:self.devices[index].path.clone(),slot:self.slot,profile,expected});
        }
    }
    fn save_local(&mut self)->bool {
        if let (Some(index),Some(mut profile))=(self.device,self.profile.clone()) {
            if self.profile_name!=profile.name(){profile.set_name(&self.profile_name);}
            match backend::save_local(&self.profile_name,&profile,&self.devices[index].firmware) {Ok(())=>{self.profile=Some(profile);self.mark_clean();self.saved=backend::saved_profiles();self.tell(crate::i18n::text("Profil auf diesem PC gespeichert."),false);self.remember(true);return true;},Err(e)=>self.tell(e,true)}
        }false
    }
    fn header(&mut self,ctx:&egui::Context) {
        egui::TopBottomPanel::top("header").frame(egui::Frame::new().fill(PANEL).inner_margin(egui::Margin::symmetric(26,16))).show(ctx,|ui|{
            ui.horizontal(|ui|{
                let (rect,_)=ui.allocate_exact_size(Vec2::splat(36.0),Sense::hover());crate::brand::paint(ui.painter(),rect);
                ui.add_space(2.0);ui.vertical(|ui|{ui.label(RichText::new("ApexControl").size(21.0).strong());small(ui,crate::i18n::text("Tastaturen & Profile"));});
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center),|ui|{
                    let gear=ui.add_enabled_ui(!self.recording,gear_button).inner;
                    if gear.clicked(){self.settings_open=true;}
                    if ui.add_enabled(!self.busy && !self.recording,secondary(crate::i18n::text("Geräte aktualisieren"))).clicked(){self.request_action(PendingAction::Scan);}
                    if self.busy {ui.spinner();}else {
                        let (label,color,hint)=self.usb_status();
                        ui.scope(|ui|pill(ui,crate::i18n::text(label),color)).response.on_hover_text(crate::i18n::text(hint));
                    }
                    if ui.add_enabled(!self.recording,secondary(&format!("Updates · {}",env!("CARGO_PKG_VERSION")))).clicked(){self.updates_open=true;}
                    if ui.add_enabled(!self.recording,secondary(crate::i18n::text("Sicherungen"))).clicked(){self.open_backups();}
                    if self.editor && ui.add(secondary(crate::i18n::text("Alle Geräte"))).clicked(){self.request_action(PendingAction::Home);}
                });
            });
        });
    }
    fn usb_status(&self)->(&'static str,Color32,&'static str){
        if self.scan_failed{return ("USB · !",ORANGE,"Die letzte Gerätesuche ist fehlgeschlagen. Bitte Geräte aktualisieren.");}
        if self.devices.is_empty(){return ("USB · –",MUTED,"Bei der letzten Suche wurde keine Apex-Tastatur gefunden.");}
        if self.devices.iter().any(|device|device.error.is_some()){
            return ("USB · !",ORANGE,"Bei mindestens einem Gerät ist der Zugriff fehlgeschlagen. Bitte Geräte aktualisieren.");
        }
        if self.devices.iter().any(|device|device.supported && !device.profiles.is_empty()){
            return ("USB bereit",GREEN,"Die USB-Kommunikation wurde bei der letzten Gerätesuche erfolgreich geprüft. Nach dem Umstecken bitte Geräte aktualisieren.");
        }
        ("Gerät erkannt",MUTED,"Ein Apex-Gerät wurde erkannt, die Konfiguration ist aber noch nicht verfügbar.")
    }
    fn unsaved_panel(&mut self,ctx:&egui::Context){
        let Some(action)=self.safety.pending.as_ref() else{return;};
        let reset=matches!(action,PendingAction::Reset);let mut choice=0;
        egui::Modal::new(egui::Id::new("unsaved_profile_changes")).show(ctx,|ui|{
            ui.set_width(480.0);ui.heading(crate::i18n::text("Ungespeicherte Profiländerungen"));
            ui.label(crate::i18n::text("Dein Entwurf wurde seit dem letzten Laden oder Speichern geändert."));
            small(ui,crate::i18n::text("Live anwenden speichert nicht dauerhaft. Bereits live übertragene Einstellungen werden beim Verwerfen nicht automatisch zurückgenommen."));
            if self.error{ui.colored_label(ORANGE,&self.status);}
            ui.add_space(12.0);ui.horizontal_wrapped(|ui|{
                if !reset && ui.add(primary(crate::i18n::text("Auf PC speichern und fortfahren"))).clicked(){choice=1;}
                if ui.add(secondary(crate::i18n::text(if reset{"Änderungen zurücksetzen"}else{"Verwerfen und fortfahren"}))).clicked(){choice=2;}
                if ui.button(crate::i18n::text("Weiter bearbeiten")).clicked(){choice=3;}
            });
            if ui.input(|input|input.key_pressed(egui::Key::Escape)){choice=3;}
        });
        match choice{1=>self.resolve_pending(true),2=>self.resolve_pending(false),3=>self.safety.pending=None,_=>{}}
    }
    fn open_backups(&mut self){
        self.backups.open=true;self.backups.device=self.device.filter(|index|self.devices.get(*index).is_some_and(|device|device.supported));
        if self.backups.device.is_none(){self.backups.device=self.devices.iter().position(|device|device.supported);}
        self.backups.slot=self.slot;self.refresh_backups();
        if let Some(slot)=self.backups.entries.first().and_then(|entry|entry.slot){self.backups.slot=slot;}
    }
    fn refresh_backups(&mut self){
        match backend::backups(){Ok((entries,rejected))=>{self.backups.entries=entries;self.backups.rejected=rejected;self.backups.error=None;self.backups.choice=if self.backups.entries.is_empty(){None}else{Some(0)};},Err(error)=>{self.backups.error=Some(error);self.backups.entries.clear();self.backups.choice=None;}}
    }
    fn backups_panel(&mut self,ctx:&egui::Context){
        if !self.backups.open || !self.settings.setup_complete(){return;}
        let mut open=true;
        egui::Window::new(crate::i18n::text("Profilsicherungen")).id(egui::Id::new("profile_backups")).open(&mut open).default_width(650.0).resizable(true).show(ctx,|ui|{
            ui.label(crate::i18n::text("Vor jedem Onboard-Schreibvorgang wird das bisherige Profil automatisch gesichert."));
            small(ui,crate::i18n::text("Diese Sicherungen enthalten Tastaturprofile, keine Programmversionen oder Firmware."));
            if ui.button(crate::i18n::text("Liste aktualisieren")).clicked(){self.refresh_backups();}
            if let Some(error)=&self.backups.error{ui.colored_label(ORANGE,error);}
            if self.backups.rejected>0{ui.colored_label(ORANGE,crate::i18n::format_text("{} ungültige Sicherungen wurden übersprungen.",&[self.backups.rejected.to_string()]));}
            ui.add_space(8.0);
            if self.backups.entries.is_empty(){ui.label(crate::i18n::text("Noch keine gültigen Profilsicherungen vorhanden."));}
            egui::ScrollArea::vertical().id_salt("backup_list").max_height(220.0).show(ui,|ui|{
                for (index,entry) in self.backups.entries.iter().enumerate(){
                    let slot=entry.slot.map(|slot|format!("Onboard {}",slot+1)).unwrap_or_else(||crate::i18n::text("Slot unbekannt").into());
                    let age=entry.modified.and_then(|time|time.elapsed().ok()).map(|age|{
                        let minutes=age.as_secs()/60;
                        if minutes<1{crate::i18n::text("Gerade eben").into()}else if minutes<60{crate::i18n::format_text("Vor {} Minuten",&[minutes.to_string()])}else if minutes<1440{crate::i18n::format_text("Vor {} Stunden",&[(minutes/60).to_string()])}else{crate::i18n::format_text("Vor {} Tagen",&[(minutes/1440).to_string()])}
                    }).unwrap_or_else(||crate::i18n::text("Zeitpunkt unbekannt").into());
                    if ui.selectable_label(self.backups.choice==Some(index),format!("{} · {} · {}",entry.profile.name(),slot,age)).on_hover_text(&entry.file_name).clicked(){self.backups.choice=Some(index);if let Some(slot)=entry.slot{self.backups.slot=slot;}}
                }
            });
            ui.separator();
            if let Some(entry)=self.backups.choice.and_then(|index|self.backups.entries.get(index)){
                small(ui,&crate::i18n::format_text("Datei: {}",&[entry.file_name.clone()]));
                ui.label(crate::i18n::text("Profilformat und Prüfsumme sind gültig."));
            }
            ui.horizontal_wrapped(|ui|{
                ui.label(crate::i18n::text("Zielgerät"));
                let devices:Vec<_>=self.devices.iter().enumerate().filter(|(_,device)|device.supported && device.error.is_none() && device.profiles.len()==5).map(|(index,device)|(index,format!("{} · {}",device.name,device.id))).collect();
                let selected=devices.iter().find(|(index,_)|Some(*index)==self.backups.device).map(|(_,name)|name.as_str()).unwrap_or(crate::i18n::text("Keine unterstützte Mini verbunden"));
                egui::ComboBox::from_id_salt("backup_device").selected_text(selected).show_ui(ui,|ui|{for (index,name) in &devices{ui.selectable_value(&mut self.backups.device,Some(*index),name);}});
                egui::ComboBox::from_id_salt("backup_slot").selected_text(format!("Slot {}",self.backups.slot+1)).show_ui(ui,|ui|{for slot in 0..5{ui.selectable_value(&mut self.backups.slot,slot,format!("Slot {}",slot+1));}});
                let ready=!self.busy && !self.recording && self.backups.choice.is_some() && devices.iter().any(|(index,_)|Some(*index)==self.backups.device);
                if ui.add_enabled(ready,primary(crate::i18n::text("Sicherung als Entwurf öffnen"))).clicked(){
                    if let (Some(index),Some(entry))=(self.backups.device,self.backups.choice.and_then(|choice|self.backups.entries.get(choice)).cloned()){
                        self.request_action(PendingAction::Backup(index,self.backups.slot,entry.profile));
                    }
                }
            });
            small(ui,crate::i18n::text("Erst als Entwurf prüfen. Zum Wiederherstellen anschließend „Auf Tastatur speichern“ klicken. Dabei entsteht erneut eine Sicherung des Zielprofils."));
        });self.backups.open&=open;
    }
    fn updates_panel(&mut self,ctx:&egui::Context){
        if !self.updates_open || !self.settings.setup_complete(){return;}
        egui::Window::new("Updates").id(egui::Id::new("app_updates")).open(&mut self.updates_open)
            .collapsible(false).resizable(true).default_width(650.0).show(ctx,|ui|{
                ui.heading("ApexControl");ui.add_space(8.0);
                ui.horizontal(|ui|{ui.label(crate::i18n::text("Installierte Version"));pill(ui,env!("CARGO_PKG_VERSION"),BLUE);});
                ui.add_space(10.0);
                egui::ScrollArea::vertical().id_salt("local_changelog").max_height(420.0).show(ui,|ui|{
                    for line in crate::changelog::text().lines(){
                        if line.starts_with("# "){continue;}
                        if let Some(title)=line.strip_prefix("## "){ui.heading(title);}
                        else if let Some(title)=line.strip_prefix("### "){ui.add_space(8.0);ui.label(RichText::new(title).strong().color(ORANGE));}
                        else if let Some(item)=line.strip_prefix("- "){ui.horizontal_top(|ui|{ui.label("•");ui.add(egui::Label::new(item).wrap());});}
                        else if !line.is_empty(){ui.add(egui::Label::new(line).wrap());}
                    }
                });
                ui.separator();
                ui.add_space(12.0);
                ui.label(crate::i18n::text("Neue Versionen und Änderungslisten findest du auf GitHub."));
                ui.label(crate::i18n::text("Der Updatekanal wird mit der ersten öffentlichen Veröffentlichung eingerichtet."));
                ui.add_space(12.0);
                if ui.add(secondary(crate::i18n::text("Veröffentlichungen auf GitHub"))).clicked(){ctx.open_url(egui::OpenUrl::new_tab(RELEASES_URL));}
                ui.add_space(8.0);
                small(ui,crate::i18n::text("Öffnet deinen Browser. Updates werden manuell heruntergeladen und installiert; eine automatische Updateprüfung ist noch nicht eingerichtet."));
            });
    }
    fn settings_panel(&mut self,ctx:&egui::Context) {
        if !self.settings_open || !self.settings.setup_complete(){return;}
        if self.autostart_poll.is_some_and(|last|last.elapsed()>=Duration::from_secs(1)){
            self.autostart_poll=Some(Instant::now());
            match desktop::autostart_enabled(){Ok(enabled)=>{self.autostart=enabled;self.autostart_error=None;},Err(error)=>self.autostart_error=Some(error)}
        }
        let mut open=true;let before=self.settings.clone();
        let detected=self.detected_keyboard_layout();
        egui::Window::new(crate::i18n::text("Einstellungen")).id(egui::Id::new("app_settings")).open(&mut open).collapsible(false).resizable(false).default_width(485.0).show(ctx,|ui|{
            ui.horizontal(|ui|{
                ui.label("Sprache / Language");
                egui::ComboBox::from_id_salt("app_language").width(160.0).selected_text(self.settings.language.label()).show_ui(ui,|ui|{
                    for language in [crate::i18n::Language::German,crate::i18n::Language::English] {ui.selectable_value(&mut self.settings.language,language,language.label());}
                });
            });ui.add_space(12.0);
            ui.label(crate::i18n::text("Tastaturlayout"));
            let selection=self.settings.keyboard_layout.map(|layout|layout.label().to_owned()).unwrap_or_else(||format!("{} · {}",crate::i18n::text("Automatisch"),detected.map(|layout|layout.label()).unwrap_or(crate::i18n::text("Nicht erkannt – bitte auswählen"))));
            egui::ComboBox::from_id_salt("keyboard_layout").width(330.0).selected_text(selection).show_ui(ui,|ui|{
                ui.selectable_value(&mut self.settings.keyboard_layout,None,crate::i18n::text("Automatisch"));
                for layout in crate::keyboard::Layout::ALL {ui.selectable_value(&mut self.settings.keyboard_layout,Some(layout),layout.label());}
            });
            small(ui,crate::i18n::text("Wähle das Layout deiner Tastenkappen. Die Windows-Eingabesprache bleibt unverändert."));ui.add_space(12.0);
            ui.heading(crate::i18n::text("Start und Fenster"));ui.add_space(10.0);
            let mut autostart=self.autostart;
            if ui.add_enabled(cfg!(windows),egui::Checkbox::new(&mut autostart,crate::i18n::text("Mit Windows starten"))).changed(){
                match desktop::set_autostart(autostart) {Ok(())=>{self.autostart=autostart;self.autostart_error=None;self.tell(crate::i18n::text("Windows-Autostart aktualisiert."),false);},Err(error)=>self.autostart_error=Some(error)}
            }
            small(ui,crate::i18n::text("Startet bei deiner Windows-Anmeldung. Die portable EXE muss dafür am selben Ort bleiben."));
            small(ui,crate::i18n::text("Nach dem ersten Einschalten findest du ApexControl auch im Task-Manager unter Autostart-Apps. Änderungen dort werden hier übernommen."));
            ui.add_enabled_ui(self.autostart && self.desktop.is_some(),|ui|{ui.checkbox(&mut self.settings.autostart_minimized,crate::i18n::text("Beim Windows-Autostart im Hintergrund starten"));});
            ui.add_space(12.0);
            ui.add_enabled_ui(self.desktop.is_some(),|ui|{ui.checkbox(&mut self.settings.minimize_on_close,crate::i18n::text("Beim Schließen in den Infobereich minimieren"));});
            small(ui,crate::i18n::text("Ein Klick auf das ApexControl-Symbol neben der Uhr (ggf. unter ^) öffnet das Fenster wieder. Rechtsklick → Beenden schließt die App vollständig."));
            if let Some(error)=&self.desktop_error {ui.label(RichText::new(crate::i18n::format_text("Infobereich nicht verfügbar: {error}", &[format!("{}", error)])).color(ORANGE));}
            if let Some(error)=&self.autostart_error {ui.label(RichText::new(crate::i18n::format_text("Autostart konnte nicht aktualisiert werden: {error}", &[format!("{}", error)])).color(ORANGE));}
            ui.add_space(14.0);ui.separator();small(ui,crate::i18n::text("Die App startet in der Geräteübersicht. Dein gemerktes Profil erscheint erst nach „Öffnen“."));
            ui.add_space(6.0);
            if ui.add(secondary(crate::i18n::text("ApexControl beenden"))).clicked(){self.quit_pending=true;}
        });
        self.settings_open=open;
        if self.settings!=before {
            if let Err(error)=self.settings.save(){self.settings=before.clone();self.tell(crate::i18n::format_text("Einstellungen konnten nicht gespeichert werden: {error}", &[format!("{}", error)]),true);}
            else if self.settings.language!=before.language {
                crate::i18n::set_language(self.settings.language);
                if let Some(desktop)=&self.desktop {desktop.update_language();}
                self.tell(crate::i18n::text("Sprache geändert."),false);ctx.request_repaint();
            }
            if self.settings.keyboard_layout!=before.keyboard_layout {
                crate::keyboard::set_layout(self.effective_keyboard_layout());self.binding_picker=None;
                self.selected.retain(|key|visible_keys().contains(key));if self.selected.is_empty(){self.selected.insert(5);}
            }
        }
    }
    fn detected_keyboard_layout(&self)->Option<crate::keyboard::Layout>{
        self.device.and_then(|index|self.devices.get(index)).and_then(|device|device.detected_layout)
            .or_else(||self.devices.iter().filter(|device|device.supported).find_map(|device|device.detected_layout))
    }
    fn effective_keyboard_layout(&self)->crate::keyboard::Layout{self.settings.keyboard_layout.or_else(||self.detected_keyboard_layout()).unwrap_or_default()}
    fn language_intro(&mut self,ctx:&egui::Context){
        if self.settings.setup_complete(){return;}
        let mut choice=None;let mut finish=false;let mut back=false;
        egui::Modal::new(egui::Id::new("first_run_language")).frame(frame()).backdrop_color(Color32::from_black_alpha(190)).show(ctx,|ui|{
            ui.set_width(430.0);
            if self.intro_language.is_none(){
                ui.label(RichText::new("Welcome to ApexControl").size(25.0).strong());ui.add_space(8.0);
                ui.label("Would you like to use ApexControl in English or German?");ui.add_space(16.0);
                ui.horizontal(|ui|{
                    let english=ui.add(primary("English").min_size(Vec2::new(190.0,48.0)));
                    if !self.intro_focus_set{english.request_focus();self.intro_focus_set=true;}
                    if english.clicked(){choice=Some(crate::i18n::Language::English);}
                    if ui.add(secondary("Deutsch").min_size(Vec2::new(190.0,48.0))).clicked(){choice=Some(crate::i18n::Language::German);}
                });ui.add_space(12.0);small(ui,"Next, choose your keyboard layout.");
            }else{
                ui.label(RichText::new(crate::i18n::text("Tastaturlayout wählen")).size(25.0).strong());ui.add_space(8.0);
                ui.label(crate::i18n::text("Welches Tastaturlayout verwendest du?"));ui.add_space(12.0);
                let response=egui::ComboBox::from_id_salt("first_run_keyboard_layout").width(330.0)
                    .selected_text(self.intro_layout.map(|layout|layout.label()).unwrap_or(crate::i18n::text("Layout auswählen …"))).show_ui(ui,|ui|{
                        for layout in crate::keyboard::Layout::ALL {ui.selectable_value(&mut self.intro_layout,Some(layout),layout.label());}
                    }).response;
                if !self.intro_focus_set{response.request_focus();self.intro_focus_set=true;}
                small(ui,crate::i18n::text("Wähle das Layout deiner Tastenkappen. Die Windows-Eingabesprache bleibt unverändert."));
                ui.add_space(12.0);ui.horizontal(|ui|{
                    if ui.add(secondary(crate::i18n::text("Zurück"))).clicked(){back=true;}
                    if ui.add_enabled(self.intro_layout.is_some(),primary(crate::i18n::text("Fertig"))).clicked(){finish=true;}
                });ui.add_space(12.0);small(ui,crate::i18n::text("Sprache und Layout kannst du später in den Einstellungen ändern."));
            }
            if let Some(error)=&self.intro_error{ui.label(RichText::new(crate::i18n::format_text("Auswahl konnte nicht gespeichert werden: {error}",&[error.clone()])).color(ORANGE));}
        });
        if let Some(language)=choice{self.intro_language=Some(language);self.intro_focus_set=false;crate::i18n::set_language(language);ctx.request_repaint();}
        if back{self.intro_language=None;self.intro_focus_set=false;self.intro_error=None;crate::i18n::set_language(crate::i18n::Language::English);ctx.request_repaint();}
        if finish{
            if let (Some(language),Some(layout))=(self.intro_language,self.intro_layout){
                match self.settings.confirm_setup(language,layout){
                    Ok(())=>{crate::i18n::set_language(language);crate::keyboard::set_layout(layout);if let Some(desktop)=&self.desktop{desktop.update_language();}self.intro_error=None;self.settings_open=false;self.tell(crate::i18n::text("Einrichtung abgeschlossen."),false);ctx.request_repaint();},
                    Err(error)=>self.intro_error=Some(error),
                }
            }
        }
    }
    fn desktop_events(&mut self,ctx:&egui::Context) {
        if self._instance.as_ref().is_some_and(|instance|instance.take_show_request()){
            self.start_hidden=None;desktop::show_window(ctx);
        }
        let events:Vec<_>=self.desktop.as_ref().map(|tray|tray.events().collect()).unwrap_or_default();
        for event in events {match event {
            Action::Show=>{self.start_hidden=None;desktop::show_window(ctx);},Action::Settings=>{self.start_hidden=None;self.stop_recording();desktop::show_window(ctx);self.settings_open=true;},Action::Quit=>self.quit_pending=true,
        }}
        if let Some(after_frame)=self.start_hidden {
            if ctx.cumulative_frame_nr()>=after_frame {self.start_hidden=None;ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));}else{ctx.request_repaint();}
        }
        if self.quit_pending && !self.busy {
            if self.allow_exit{ctx.send_viewport_cmd(egui::ViewportCommand::Close);}else{self.quit_pending=false;self.request_action(PendingAction::Quit);}
        }
        if ctx.input(|input|input.viewport().close_requested()) && !self.allow_exit {
            let preferences=if self.settings.setup_complete(){self.settings.clone()}else{Settings::default()};
            match desktop::close_action(&preferences,self.desktop.is_some(),self.busy,false) {
                CloseAction::Wait=>{ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);self.tell(crate::i18n::text("Die Tastatur wird noch bearbeitet. Bitte anschließend erneut schließen."),false);},
                CloseAction::Hide=>{ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);self.stop_recording();self.remember(true);ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));},
                CloseAction::Exit=>{if self.dirty(){ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);self.request_action(PendingAction::Quit);}},
            }
        }
    }
    fn home(&mut self,ui:&mut egui::Ui) {
        ui.add_space(18.0);ui.label(RichText::new(crate::i18n::text("Deine Tastaturen")).size(29.0).strong());
        ui.label(RichText::new(crate::i18n::text("Wähle eine Tastatur oder lade direkt ein Profil.")).color(MUTED));
        small(ui,crate::i18n::text("SteelSeries GG vor Änderungen vollständig beenden, damit die Einstellungen erhalten bleiben."));ui.add_space(20.0);
        if self.devices.is_empty() {
            card(ui,|ui|{ui.set_min_height(180.0);ui.vertical_centered(|ui|{ui.add_space(40.0);ui.label(RichText::new(if self.busy{crate::i18n::text("Tastaturen werden gesucht …")}else{crate::i18n::text("Verbinde deine Apex per USB")}).size(22.0));ui.add_space(12.0);small(ui,crate::i18n::text("Deine gespeicherten Profile bleiben auf diesem PC verfügbar."));});});return;
        }
        let width=((ui.available_width()-20.0)/2.0).min(620.0);
        for row in 0..self.devices.len().div_ceil(2) {
            ui.horizontal_top(|ui|{for index in row*2..((row+1)*2).min(self.devices.len()) {
                let d=self.devices[index].clone();
                card(ui,|ui|{
                    ui.set_width(width-38.0);
                    ui.horizontal(|ui|{ui.label(RichText::new(&d.name).size(21.0).strong());ui.with_layout(egui::Layout::right_to_left(egui::Align::Center),|ui|{pill(ui,if d.supported{crate::i18n::text("Verbunden")}else{crate::i18n::text("Erkannt")},if d.supported{GREEN}else{MUTED});});});
                    ui.add_space(12.0);
                    let (rect,response)=ui.allocate_exact_size(Vec2::new(ui.available_width(),118.0),Sense::click());
                    keyboard_thumbnail(ui.painter(),rect,&d.name);
                    if response.clicked(){self.open_device(index);}
                    ui.add_space(10.0);
                    if d.supported {
                        if let Some(e)=&d.error {ui.label(RichText::new(e).color(ORANGE).size(12.0));}
                        let choice=self.card_choices[index];
                        let label=if choice<5 {d.profiles.get(choice).map(|p|format!("Onboard {} · {}",choice+1,p.name())).unwrap_or(crate::i18n::text("Profil wird gelesen …").into())}else{self.saved.get(choice-100).map(|p|crate::i18n::format_text("Auf PC · {}", &[format!("{}", p.name)])).unwrap_or(crate::i18n::text("Profil wählen").into())};
                        egui::ComboBox::from_id_salt(("card_profile",index)).width(ui.available_width()-5.0).selected_text(label).show_ui(ui,|ui|{
                            for (slot,p) in d.profiles.iter().enumerate(){ui.selectable_value(&mut self.card_choices[index],slot,format!("Onboard {} · {}",slot+1,p.name()));}
                            if !self.saved.is_empty(){ui.separator();for (i,p) in self.saved.iter().enumerate(){ui.selectable_value(&mut self.card_choices[index],100+i,crate::i18n::format_text("Auf PC · {}", &[format!("{}", p.name)]));}}
                        });
                        ui.add_space(6.0);ui.horizontal_wrapped(|ui|{
                            if ui.add_enabled(!self.busy && d.profiles.len()==5,primary(crate::i18n::text("Profil laden"))).clicked(){let choice=self.card_choices[index];if choice<5 {self.load_slot(index,choice as u8);}else{self.local_draft(index,choice-100);}}
                            if ui.add_enabled(!self.busy,secondary(crate::i18n::text("Öffnen"))).clicked(){self.open_device(index);}
                        });
                    }else {small(ui,crate::i18n::text("Dieses Modell wird erkannt. Konfiguration folgt mit eigener Geräteprüfung."));}
                });
            }});ui.add_space(16.0);
        }
    }
    fn sidebar(&mut self,ctx:&egui::Context,index:usize) {
        let d=self.devices[index].clone();
        egui::SidePanel::left("profiles").exact_width(235.0).resizable(false).frame(egui::Frame::new().fill(PANEL).inner_margin(18)).show(ctx,|ui|{
            ui.label(RichText::new(&d.name).size(18.0).strong());small(ui,crate::i18n::text("ONBOARD-PROFILE"));ui.add_space(12.0);
            for (slot,p) in d.profiles.iter().enumerate() {
                let active=self.profile.is_some() && self.slot==slot as u8;
                let text=RichText::new(format!("{}    {}",slot+1,p.name())).color(if active{ORANGE}else{TEXT});
                if ui.add_enabled(!self.busy && !self.recording,egui::Button::new(text).fill(if active{ORANGE.gamma_multiply(0.1)}else{PANEL}).stroke(if active{Stroke::new(1.0_f32,ORANGE.gamma_multiply(0.45))}else{Stroke::NONE}).min_size(Vec2::new(198.0,40.0)).corner_radius(7)).clicked(){self.load_slot(index,slot as u8);}
            }
            ui.add_space(22.0);small(ui,crate::i18n::text("AUF DIESEM PC"));ui.add_space(8.0);
            if self.saved.is_empty(){small(ui,crate::i18n::text("Noch keine gespeicherten Profile."));}
            let locals:Vec<_>=self.saved.iter().enumerate().filter(|(_,p)|p.pid==d.pid).map(|(i,p)|(i,p.name.clone())).collect();
            for (i,name) in locals {if ui.add_enabled(!self.busy && !self.recording,egui::Button::new(name).fill(PANEL).min_size(Vec2::new(198.0,32.0))).clicked(){self.local_draft(index,i);}}
            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT),|ui|{small(ui,&format!("Firmware {}",if d.firmware.is_empty(){"–"}else{&d.firmware}));small(ui,"ApexControl 0.1.0");});
        });
    }
    fn footer(&mut self,ctx:&egui::Context) {
        egui::TopBottomPanel::bottom("actions").frame(egui::Frame::new().fill(PANEL).inner_margin(egui::Margin::symmetric(22,14))).show(ctx,|ui|{
            if self.tab==Tab::Rgb {small(ui,crate::i18n::text("RGB wird live übernommen. Mit „Onboard-Beleuchtung“ kehrst du zum gespeicherten Effekt zurück."));return;}
            ui.horizontal(|ui|{
                let enabled=self.profile.is_some() && !self.busy && !self.recording;
                ui.add_enabled_ui(enabled,|ui|{
                    if ui.add(secondary(crate::i18n::text("Live anwenden"))).clicked(){self.apply();}
                    ui.add_space(6.0);ui.checkbox(&mut self.live_preview,crate::i18n::text("Live-Vorschau"));
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center),|ui|{
                    if ui.add_enabled(enabled,primary(crate::i18n::text("Auf Tastatur speichern"))).clicked(){self.store();}
                    if ui.add_enabled(enabled,secondary(crate::i18n::text("Auf PC speichern"))).clicked(){self.save_local();}
                    if enabled {egui::ComboBox::from_id_salt("target_slot").width(85.0).selected_text(format!("Slot {}",self.slot+1)).show_ui(ui,|ui|{
                        for slot in 0..5 {if ui.selectable_value(&mut self.slot,slot,format!("Slot {}",slot+1)).changed(){if let Some(index)=self.device {self.baseline=self.devices[index].profiles.get(slot as usize).cloned();}}}
                    });}
                });
            });
        });
    }
    fn editor(&mut self,ui:&mut egui::Ui,index:usize) {
        let d=self.devices[index].clone();
        if !d.supported {ui.add_space(25.0);ui.heading(&d.name);ui.label(crate::i18n::text("Die Tastatur wird erkannt. Für dieses Modell sind noch keine geprüften Schreibbefehle freigeschaltet."));return;}
        ui.add_space(8.0);
        if self.profile.is_none() {
            ui.heading(&d.name);ui.add_space(16.0);card(ui,|ui|{ui.heading(crate::i18n::text("Mit einem Profil starten"));ui.label(crate::i18n::text("Wähle links eines der fünf Onboard-Profile. Es wird direkt von deiner Tastatur geladen."));
                keyboard_thumbnail(ui.painter(),ui.available_rect_before_wrap().with_max_y(ui.cursor().min.y+150.0),&d.name);ui.add_space(170.0);
                if let Some(e)=&d.error {ui.label(RichText::new(e).color(ORANGE));}
            });return;
        }
        ui.horizontal(|ui|{
            ui.vertical(|ui|{ui.label(RichText::new(crate::i18n::text("Tastatur konfigurieren")).size(25.0).strong());small(ui,&format!("{} · {}",d.name,self.effective_keyboard_layout().label()));});
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center),|ui|{
                    if self.dirty(){pill(ui,crate::i18n::text("Ungespeicherte Änderungen"),ORANGE);}
                if let (Some(p),Some(b))=(&self.profile,&self.baseline) {let saved=p==b && self.profile_name==b.name();pill(ui,if saved{crate::i18n::text("Auf Tastatur gespeichert")}else{crate::i18n::text("Nicht auf Tastatur gespeichert")},if saved{GREEN}else{ORANGE});}
            });
        });ui.add_space(12.0);
        ui.horizontal(|ui|{
            for (tab,title) in [(Tab::Actuation,crate::i18n::text("Betätigung")),(Tab::Bindings,crate::i18n::text("Tastenbelegung")),(Tab::Macros,crate::i18n::text("Makros")),(Tab::Rgb,crate::i18n::text("Beleuchtung"))] {
                let active=self.tab==tab;
                let response=ui.add(egui::Button::new(RichText::new(title).color(if active{ORANGE}else{MUTED}).strong()).fill(if active{CARD}else{BG}).stroke(if active{Stroke::new(1.0_f32,LINE)}else{Stroke::NONE}).corner_radius(7).min_size(Vec2::new(135.0,35.0)));
                if response.clicked() && !self.recording {self.tab=tab;}
            }
        });ui.add_space(14.0);
        card(ui,|ui|{
            ui.horizontal(|ui|{
                ui.label(RichText::new(if self.selected.len()==1 {crate::i18n::format_text("Taste {}", &[format!("{}", protocol::key_label(*self.selected.first().unwrap()))])}else{crate::i18n::format_text("{} Tasten ausgewählt", &[format!("{}", self.selected.len())])}).strong());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center),|ui|{
                    if ui.small_button(crate::i18n::text("Alle auswählen")).clicked(){self.selected=visible_keys().into_iter().collect();}
                    small(ui,crate::i18n::text("Umschalt + Klick für mehrere Tasten"));
                });
            });ui.add_space(10.0);
            if self.tab==Tab::Rgb{
                let colors=self.rgb_ui.colors.entry(d.id.clone()).or_default();
                keyboard(ui,self.profile.as_ref().unwrap(),&mut self.selected,Some(colors),&mut self.keyboard_interaction);
            }else{keyboard(ui,self.profile.as_ref().unwrap(),&mut self.selected,None,&mut self.keyboard_interaction);}
            ui.add_space(5.0);
            ui.horizontal_wrapped(|ui|{if self.tab!=Tab::Rgb{pill(ui,"Rapid Trigger",ORANGE);pill(ui,crate::i18n::text("Schutzmodus"),BLUE);}small(ui,crate::i18n::text("Shift + Klick: mehrere Tasten. Shift + Ziehen: Auswahlbox. Strg + Shift: zur Auswahl hinzufügen."));});
        });ui.add_space(14.0);
        match self.tab {Tab::Actuation=>self.actuation_controls(ui),Tab::Bindings=>self.binding_controls(ui),Tab::Macros=>self.macro_controls(ui),Tab::Rgb=>self.rgb_controls(ui)}
        ui.add_space(12.0);
        ui.horizontal(|ui|{small(ui,crate::i18n::text("Profilname"));ui.add_enabled(!self.recording,egui::TextEdit::singleline(&mut self.profile_name).desired_width(235.0));
            if ui.add_enabled(!self.busy && !self.recording && self.dirty(),egui::Button::new(crate::i18n::text("Änderungen zurücksetzen"))).clicked(){self.request_action(PendingAction::Reset);}
            if self.dirty(){pill(ui,crate::i18n::text("Ungespeicherte Änderungen"),ORANGE);}else{small(ui,crate::i18n::text("Keine ungespeicherten Änderungen"));}
        });
    }
    fn actuation_controls(&mut self,ui:&mut egui::Ui) {
        let keys:Vec<_>=self.selected.iter().copied().collect();let Some(key)=keys.first() else{return};
        let Some(state)=self.profile.as_ref().and_then(|p|p.value(*key)) else{return};
        let width=(ui.available_width()-24.0)/3.0;let mut edited=false;let mut finished=false;
        ui.horizontal_top(|ui|{
            card(ui,|ui|{ui.set_width(width-38.0);ui.label(RichText::new(crate::i18n::text("Auslösepunkt")).strong());small(ui,crate::i18n::text("Wie tief die Taste gedrückt werden muss."));ui.add_space(8.0);
                let mut value=state.tenths().unwrap_or(18) as i32;ui.label(RichText::new(format!("{:.1} mm",value as f32/10.0)).size(25.0).color(ORANGE));
                let response=ui.add(egui::Slider::new(&mut value,1..=40).show_value(false));
                if response.changed(){if let Some(p)=&mut self.profile {let _=p.set_actuation(&keys,value as u8);}edited=true;}finished|=response.drag_stopped() || (response.changed() && !response.dragged());
                ui.horizontal(|ui|{small(ui,crate::i18n::text("0,1 mm"));ui.with_layout(egui::Layout::right_to_left(egui::Align::Center),|ui|small(ui,crate::i18n::text("4,0 mm")));});
            });
            card(ui,|ui|{ui.set_width(width-38.0);ui.horizontal(|ui|{lightning(ui,ORANGE);ui.label(RichText::new("Rapid Trigger").strong());});
                let mut enabled=state.rapid();let mut sens=state.sensitivity.clamp(1,40) as i32;
                if ui.checkbox(&mut enabled,crate::i18n::text("Aktivieren")).changed(){if let Some(p)=&mut self.profile {p.set_rapid(&keys,enabled);}edited=true;finished=true;}
                ui.add_enabled_ui(enabled,|ui|{small(ui,crate::i18n::text("Empfindlichkeit"));let r=ui.add(egui::Slider::new(&mut sens,1..=40).custom_formatter(|v,_|format!("{:.1} mm",v/10.0)));
                    if r.changed(){if let Some(p)=&mut self.profile {p.set_sensitivity(&keys,sens as u8);}edited=true;}finished|=r.drag_stopped() || (r.changed()&&!r.dragged());});
                small(ui,crate::i18n::text("Dynamisches Auslösen und Zurücksetzen."));
            });
            card(ui,|ui|{ui.set_width(width-38.0);ui.horizontal(|ui|{shield(ui,BLUE);ui.label(RichText::new(crate::i18n::text("Schutzmodus")).strong());});
                let mut enabled=state.protection();if ui.checkbox(&mut enabled,crate::i18n::text("Aktivieren")).changed(){if let Some(p)=&mut self.profile {p.set_protection(&keys,enabled);}edited=true;finished=true;}
                ui.add_space(7.0);small(ui,crate::i18n::text("Protection Mode für die ausgewählten Tasten."));small(ui,crate::i18n::text("Mit Rapid Trigger kombinierbar."));
            });
        });
        if edited {self.tell(crate::i18n::text("Änderung im Profilentwurf. Live anwenden oder auf der Tastatur speichern."),false);}
        if finished && self.live_preview && !self.busy {self.apply();}
    }
    fn binding_controls(&mut self,ui:&mut egui::Ui) {
        let keys:Vec<_>=self.selected.iter().copied().collect();let Some(key)=keys.first() else{return};
        let Some(profile)=self.profile.as_ref() else{return};
        let mut picker=self.binding_picker.take().filter(|picker|picker.matches(profile,&keys)).unwrap_or_else(||BindingPicker::for_selection(profile,keys.clone()));
        let source=if keys.len()==1{protocol::key_label(*key)}else{crate::i18n::format_text("{} Tasten", &[format!("{}", keys.len())])};
        let current=profile.value(*key).map(|state|match state.binding[0]{0x51=>protocol::key_label(state.binding[1]),0x71=>crate::i18n::text("Makro").into(),_=>crate::i18n::text("Gerätefunktion").into()}).unwrap_or_default();
        card(ui,|ui|{
            ui.heading(crate::i18n::text("Eine Taste umbelegen"));small(ui,crate::i18n::text("1. Taste oben anklicken.  2. Gewünschte Ausgabe wählen.  3. Zuweisen."));ui.add_space(8.0);
            ui.label(RichText::new(format!("{source} → {}",protocol::key_label(picker.target))).size(25.0).strong().color(ORANGE));
            small(ui,&crate::i18n::format_text("Aktuelle Ausgabe: {current}", &[format!("{}", current)]));ui.add_space(8.0);
            ui.add_enabled_ui(!self.busy && !self.recording,|ui|{ui.horizontal_wrapped(|ui|{
                let category=picker.category;
                egui::ComboBox::from_id_salt("binding_category").width(155.0).selected_text(picker.category.label()).show_ui(ui,|ui|{
                    for category in KeyCategory::ALL {ui.selectable_value(&mut picker.category,category,category.label());}
                });
                if picker.category!=category {picker.target=picker.category.keys()[0];}
                egui::ComboBox::from_id_salt("binding_target").width(160.0).selected_text(protocol::key_label(picker.target)).show_ui(ui,|ui|{
                    for hid in picker.category.keys(){ui.selectable_value(&mut picker.target,hid,protocol::key_label(hid));}
                });
                if ui.add(primary(crate::i18n::text("Zuweisen"))).clicked(){self.profile.as_mut().unwrap().set_binding(&keys,picker.target);self.tell(crate::i18n::format_text("{source} → {} zugewiesen. Zum Aktivieren auf der Tastatur speichern.", &[format!("{}", source), format!("{}", protocol::key_label(picker.target))]),false);}
                if ui.add(secondary(crate::i18n::text("Originaltaste"))).clicked(){self.profile.as_mut().unwrap().reset_binding(&keys);self.tell(crate::i18n::text("Originalbelegung wiederhergestellt. Zum Aktivieren auf der Tastatur speichern."),false);}
            });});
            small(ui,crate::i18n::text("Zum Aktivieren anschließend unten „Auf Tastatur speichern“ klicken. Ein normales Umbelegen benötigt keine Makroaufnahme."));
            });
        self.binding_picker=Some(picker);
    }
    fn rgb_controls(&mut self,ui:&mut egui::Ui) {
        card(ui,|ui|{
            ui.heading(crate::i18n::text("Beleuchtung"));small(ui,crate::i18n::text("RGB wird nur auf die markierten Tasten angewendet. Für die ganze Tastatur „Alle auswählen“ verwenden."));
            ui.add_space(10.0);
            ui.horizontal_wrapped(|ui|{
                ui.label(crate::i18n::text("Modus"));
                egui::ComboBox::from_id_salt("rgb_mode").selected_text(self.rgb_ui.mode.label()).show_ui(ui,|ui|{for mode in crate::rgb::Mode::ALL{ui.selectable_value(&mut self.rgb_ui.mode,mode,mode.label());}});
                if self.rgb_ui.mode!=crate::rgb::Mode::Static{ui.label(crate::i18n::text("Geschwindigkeit"));ui.add(egui::Slider::new(&mut self.rgb_ui.speed,0.2..=3.0).suffix("×"));}
            });
            ui.horizontal_wrapped(|ui|{
                ui.add_enabled_ui(self.rgb_ui.mode!=crate::rgb::Mode::Wave,|ui|{ui.horizontal(|ui|{ui.label(crate::i18n::text("Farbe"));ui.color_edit_button_srgb(&mut self.rgb_color);ui.label(format!("#{:02X}{:02X}{:02X}",self.rgb_color[0],self.rgb_color[1],self.rgb_color[2]));});});
                ui.add_space(15.0);ui.label(crate::i18n::text("Helligkeit"));
                ui.add(egui::Slider::new(&mut self.rgb_brightness,0..=100).suffix(" %"));
            });
            ui.add_enabled_ui(self.rgb_ui.mode!=crate::rgb::Mode::Wave,|ui|{ui.horizontal_wrapped(|ui|{
                for (name,color) in [("Orange",[255,143,79]),(crate::i18n::text("Weiß"),[255,255,255]),(crate::i18n::text("Blau"),[65,115,255]),(crate::i18n::text("Lila"),[170,70,255]),(crate::i18n::text("Grün"),[45,220,105]),(crate::i18n::text("Rot"),[255,40,45])] {
                    if ui.button(RichText::new(name).color(Color32::from_rgb(color[0],color[1],color[2]))).clicked(){self.rgb_color=color;}
                }
            });});
            small(ui,&crate::i18n::format_text("{} Tasten ausgewählt",&[self.selected.len().to_string()]));
            ui.horizontal_wrapped(|ui|{
                if ui.add_enabled(!self.busy && !self.selected.is_empty(),primary(crate::i18n::text("Auf Auswahl anwenden"))).clicked(){self.apply_rgb_selection();}
                if ui.add_enabled(!self.busy,secondary(crate::i18n::text("Onboard-Beleuchtung"))).clicked(){if let Some(index)=self.device {self.rgb_ui.pending_device=Some(self.devices[index].id.clone());self.command(Request::ClearRgb{path:self.devices[index].path.clone()});}}
            });
            if self.rgb_ui.mode!=crate::rgb::Mode::Static{small(ui,crate::i18n::text("Software-Effekte laufen, solange ApexControl geöffnet ist, auch im Infobereich. Sie werden nicht im Onboard-Profil gespeichert."));}
            if self.rgb_ui.mode==crate::rgb::Mode::Ripple{small(ui,crate::i18n::text("Reaktiv nutzt unter Windows nur Tastensignale der Apex Pro Mini, ohne Aufzeichnung. Umbelegungen und Fn können den Ursprung verschieben."));}
            small(ui,crate::i18n::text("Direkte RGB-Farben sind flüchtig. Mit „Onboard-Beleuchtung“ stellst du den gespeicherten Effekt wieder her."));
        });
    }
    fn apply_rgb_selection(&mut self){
        if self.busy{return;}let Some(device)=self.device.and_then(|index|self.devices.get(index)).filter(|device|device.supported) else{return;};
        let origins=crate::rgb::positions(self.effective_keyboard_layout());let keys=origins.iter().filter(|point|self.selected.contains(&point.hid)).copied().collect();
        let config=crate::rgb::Config{mode:self.rgb_ui.mode,color:self.rgb_color,brightness:self.rgb_brightness,speed:self.rgb_ui.speed,keys,origins};
        let path=device.path.clone();let id=device.id.clone();
        let mut effect=match crate::rgb::Effect::new(config.clone()){Ok(effect)=>effect,Err(error)=>{self.tell(error,true);return;}};
        self.rgb_ui.pending_device=Some(id);
        if config.mode==crate::rgb::Mode::Static{
            let frame=effect.frame(0.0);let keys=frame.iter().map(|(hid,_)|*hid).collect();let color=frame[0].1;
            self.rgb_ui.pending_frame=Some(frame);self.command(Request::Rgb{path,keys,color});
        }else{self.command(Request::RgbEffect{path,device_id:device.id.clone(),config,preview:self.rgb_ui.preview.clone()});}
    }
    fn record_events(&mut self,ctx:&egui::Context) {
        if !self.recording {return;}
        for event in ctx.input(|i|i.events.clone()) {
            if let egui::Event::Key{key,physical_key,pressed,repeat,..}=event {
                if repeat {continue;}let Some(hid)=event_hid(physical_key.unwrap_or(key)) else{continue};
                if pressed && self.held.contains(&hid) || !pressed && !self.held.contains(&hid) {continue;}
                let now=Instant::now();let pause=self.last_event.map(|previous|now.duration_since(previous).as_millis().min(65535) as u16).filter(|ms|*ms>0);
                // Reserve space for releasing every held key, even when many
                // key events arrive in a single frame.
                let held_after=if pressed{self.held.len()+1}else{self.held.len()-1};
                if self.recorded.len()+usize::from(pause.is_some())+1+held_after>1000 {
                    self.stop_recording();self.tell(crate::i18n::text("Die Aufnahme wurde bei der maximalen Länge gestoppt."),false);break;
                }
                if let Some(ms)=pause{self.recorded.push(MacroEvent::Delay{ms});}
                self.recorded.push(MacroEvent::Key{hid,down:pressed});if pressed {self.held.insert(hid);}else{self.held.remove(&hid);}self.last_event=Some(now);
            }
        }
        if self.recorded.len()>950 {self.stop_recording();self.tell(crate::i18n::text("Die Aufnahme wurde bei der maximalen Länge gestoppt."),false);}
    }
    fn stop_recording(&mut self) {for hid in std::mem::take(&mut self.held){self.recorded.push(MacroEvent::Key{hid,down:false});}self.recording=false;self.last_event=None;}
    fn macro_controls(&mut self,ui:&mut egui::Ui) {
        let keys:Vec<_>=self.selected.iter().copied().collect();
        card(ui,|ui|{
            ui.horizontal(|ui|{ui.heading(crate::i18n::text("Makro-Editor"));ui.with_layout(egui::Layout::right_to_left(egui::Align::Center),|ui|{pill(ui,if self.recording{crate::i18n::text("Aufzeichnung läuft")}else{crate::i18n::text("Tastaturmakro")},if self.recording{ORANGE}else{MUTED});});});
            ui.horizontal_wrapped(|ui|{small(ui,crate::i18n::text("Nur eine Taste ändern, z. B. B → I?"));if ui.add_enabled(!self.recording,secondary(crate::i18n::text("Tastenbelegung öffnen"))).clicked(){self.tab=Tab::Bindings;}});
            ui.add_space(8.0);
            ui.add_enabled_ui(!self.recording,|ui|{
                ui.label(RichText::new(crate::i18n::text("Tasten manuell einfügen")).strong());
                ui.horizontal_wrapped(|ui|{
                    let before=self.macro_category;
                    egui::ComboBox::from_id_salt("macro_key_category").width(150.0).selected_text(self.macro_category.label()).show_ui(ui,|ui|{
                        for category in KeyCategory::ALL {ui.selectable_value(&mut self.macro_category,category,category.label());}
                    });
                    if self.macro_category!=before {self.macro_key=self.macro_category.keys()[0];}
                    egui::ComboBox::from_id_salt("macro_insert_key").width(140.0).selected_text(protocol::key_label(self.macro_key)).show_ui(ui,|ui|{
                        for hid in self.macro_category.keys(){ui.selectable_value(&mut self.macro_key,hid,protocol::key_label(hid));}
                    });
                    egui::ComboBox::from_id_salt("macro_key_action").width(155.0).selected_text([crate::i18n::text("Drücken + Loslassen"),crate::i18n::text("Nur drücken"),crate::i18n::text("Nur loslassen")][self.macro_action as usize]).show_ui(ui,|ui|{
                        for (action,label) in [crate::i18n::text("Drücken + Loslassen"),crate::i18n::text("Nur drücken"),crate::i18n::text("Nur loslassen")].iter().enumerate(){ui.selectable_value(&mut self.macro_action,action as u8,*label);}
                    });
                    if ui.add(secondary(crate::i18n::text("Taste hinzufügen"))).clicked(){match protocol::insert_macro_key(&mut self.recorded,self.macro_key,self.macro_action){Ok(())=>self.tell(crate::i18n::text("Taste im Makro eingefügt."),false),Err(error)=>self.tell(error,true)}}
                });
                ui.horizontal_wrapped(|ui|{
                    ui.label("Pause");ui.add(egui::DragValue::new(&mut self.macro_delay).range(0..=65535).suffix(" ms"));
                    if ui.button(crate::i18n::text("Pause hinzufügen")).clicked(){if self.recorded.len()<1000 {self.recorded.push(MacroEvent::Delay{ms:self.macro_delay});}else{self.tell(crate::i18n::text("Das Makro hat die maximale Länge erreicht."),true);}}
                    if ui.add_enabled(!self.recorded.is_empty(),egui::Button::new(crate::i18n::text("Letztes Ereignis entfernen"))).clicked(){self.recorded.pop();}
                    if ui.add_enabled(!self.recorded.is_empty(),egui::Button::new(crate::i18n::text("Makro leeren"))).clicked(){self.recorded.clear();}
                });
                small(ui,crate::i18n::text("Zum Beispiel Strg + C: Strg „Nur drücken“, C „Drücken + Loslassen“, Strg „Nur loslassen“. Zeichen folgen dem Windows-Tastaturlayout."));
            });
            ui.add_space(8.0);ui.horizontal(|ui|{
                ui.add_enabled(!self.recording,egui::TextEdit::singleline(&mut self.macro_name).desired_width(180.0).hint_text(crate::i18n::text("Makroname")));
                if self.recording {if ui.add(primary(crate::i18n::text("Aufnahme stoppen"))).clicked(){self.stop_recording();}}else if ui.add(secondary(crate::i18n::text("Aufnahme starten"))).clicked(){self.recorded.clear();self.recording=true;self.held.clear();self.last_event=None;}
                if ui.add_enabled(!self.recording && !self.recorded.is_empty(),secondary(crate::i18n::text("Makro auf PC speichern"))).clicked(){
                    let entry=MacroRecord{name:self.macro_name.clone(),events:self.recorded.clone()};
                    match backend::save_macro(&self.library,&entry){Ok(())=>{self.library.push(entry);self.tell(crate::i18n::text("Makro auf diesem PC gespeichert."),false);},Err(e)=>self.tell(e,true)}
                }
            });
            small(ui,if self.recording{crate::i18n::text("Drücke jetzt die gewünschten Tasten. Die Pausen werden mit aufgezeichnet.")}else{crate::i18n::text("Buchstaben, Zahlen, Funktionstasten und Pausen. Wiedergabe einmal beim Drücken.")});
            ui.add_space(8.0);
            egui::ScrollArea::horizontal().max_height(70.0).show(ui,|ui|{ui.horizontal(|ui|{
                if self.recorded.is_empty(){small(ui,crate::i18n::text("Noch keine Ereignisse aufgenommen."));}
                for event in &self.recorded {match event {MacroEvent::Key{hid,down}=>pill(ui,&format!("{} {}",protocol::key_label(*hid),if *down{"↓"}else{"↑"}),if *down{ORANGE}else{MUTED}),MacroEvent::Delay{ms}=>{small(ui,&format!("{ms} ms"));}}}
            });});
            ui.add_space(6.0);
            if !self.library.is_empty(){ui.horizontal(|ui|{egui::ComboBox::from_id_salt("macro_library").selected_text(self.library.get(self.macro_choice).map(|m|m.name.as_str()).unwrap_or(crate::i18n::text("Makro wählen"))).show_ui(ui,|ui|{for (i,m) in self.library.iter().enumerate(){ui.selectable_value(&mut self.macro_choice,i,&m.name);}});if ui.button(crate::i18n::text("In Editor laden")).clicked(){if let Some(m)=self.library.get(self.macro_choice){self.recorded=m.events.clone();self.macro_name=m.name.clone();}}});}
            if ui.add_enabled(!self.recording && !self.recorded.is_empty(),secondary(crate::i18n::text("Den ausgewählten Tasten zuweisen"))).clicked(){
                match self.profile.as_mut().unwrap().assign_first_macro(&keys,&self.recorded){Ok(())=>self.tell(crate::i18n::text("Makro zugewiesen. Auf Tastatur speichern, um es zu übernehmen."),false),Err(e)=>self.tell(e,true)}
            }
            let native=self.profile.as_ref().unwrap().macros();if !native.is_empty(){small(ui,&crate::i18n::format_text("{} natives Makro im Profil. Vorhandene Makrodaten werden geschützt.", &[format!("{}", native.len())]));}
        });
    }
}

impl eframe::App for ApexApp {
    fn update(&mut self,ctx:&egui::Context,_:&mut eframe::Frame) {
        self.poll();crate::keyboard::set_layout(self.effective_keyboard_layout());self.desktop_events(ctx);self.record_events(ctx);self.header(ctx);
        if self.editor {self.footer(ctx);if let Some(index)=self.device {if index<self.devices.len(){self.sidebar(ctx,index);}}}
        egui::TopBottomPanel::bottom("status").frame(egui::Frame::new().fill(BG).inner_margin(egui::Margin::symmetric(24,9))).show(ctx,|ui|{
            ui.horizontal(|ui|{if self.busy {ui.spinner();}else{ui.painter().circle_filled(ui.cursor().min+Vec2::new(5.0,9.0),3.0,if self.error{ORANGE}else{GREEN});ui.add_space(13.0);}ui.label(RichText::new(&self.status).size(12.0).color(if self.error{ORANGE}else{MUTED}));});
        });
        self.settings_panel(ctx);self.updates_panel(ctx);
        egui::CentralPanel::default().frame(egui::Frame::new().fill(BG).inner_margin(egui::Margin::symmetric(24,12))).show(ctx,|ui|{
            egui::ScrollArea::vertical().show(ui,|ui|{if self.editor {if let Some(index)=self.device {if index<self.devices.len(){ui.add_enabled_ui(!self.busy,|ui|self.editor(ui,index));}}}else{self.home(ui);}});
        });
        self.backups_panel(ctx);self.unsaved_panel(ctx);self.language_intro(ctx);
        ctx.request_repaint_after(if self.rgb_ui.active.is_some(){Duration::from_millis(33)}else if self.busy || self.recording {Duration::from_millis(40)}else{Duration::from_millis(250)});
    }
}

fn lightning(ui:&mut egui::Ui,color:Color32) {let (r,_)=ui.allocate_exact_size(Vec2::new(16.0,20.0),Sense::hover());let p=ui.painter();p.add(egui::Shape::convex_polygon(vec![r.min+Vec2::new(10.0,1.0),r.min+Vec2::new(3.0,11.0),r.min+Vec2::new(8.0,11.0),r.min+Vec2::new(6.0,19.0),r.min+Vec2::new(14.0,8.0),r.min+Vec2::new(9.0,8.0)],color,Stroke::NONE));}
fn shield(ui:&mut egui::Ui,color:Color32) {let (r,_)=ui.allocate_exact_size(Vec2::new(17.0,20.0),Sense::hover());ui.painter().add(egui::Shape::convex_polygon(vec![r.min+Vec2::new(8.0,1.0),r.min+Vec2::new(15.0,4.0),r.min+Vec2::new(14.0,12.0),r.min+Vec2::new(8.0,19.0),r.min+Vec2::new(2.0,12.0),r.min+Vec2::new(1.0,4.0)],color.gamma_multiply(0.25),Stroke::new(1.4_f32,color)));}

fn rows() -> Vec<Vec<(u8,f32)>> {crate::keyboard::current().rows()}
fn visible_keys()->Vec<u8>{rows().into_iter().flatten().map(|(h,_)|h).collect::<BTreeSet<_>>().into_iter().collect()}
fn key_regions(area:Rect,unit:f32,height:f32)->Vec<(usize,u8,Rect)>{
    let mut regions=Vec::new();for (row,keys) in rows().iter().enumerate(){let mut x=area.min.x+9.0;for (hid,span) in keys{
        let rect=Rect::from_min_size(Pos2::new(x,area.min.y+9.0+row as f32*height),Vec2::new(unit*span-4.0,height-4.0));x+=unit*span;regions.push((row,*hid,rect));
    }}regions
}
fn contrast(color:Color32)->Color32{if (color.r() as u32*299+color.g() as u32*587+color.b() as u32*114)>145000{Color32::BLACK}else{Color32::WHITE}}
fn keyboard(ui:&mut egui::Ui,profile:&Profile,selected:&mut BTreeSet<u8>,colors:Option<&BTreeMap<u8,[u8;3]>>,interaction:&mut KeyboardInteraction) {
    let width=ui.available_width().min(1050.0);let unit=(width-18.0)/15.0;let height=unit*0.78;
    let (area,_)=ui.allocate_exact_size(Vec2::new(width,height*5.0+18.0),Sense::click_and_drag());
    let regions=key_regions(area,unit,height);
    let (pos,pressed,released,down,shift,ctrl)=ui.input(|input|(input.pointer.interact_pos(),input.pointer.primary_pressed(),input.pointer.primary_released(),input.pointer.primary_down(),input.modifiers.shift,input.modifiers.ctrl));
    if !down && !released{interaction.marquee=None;}
    if pressed && shift && ui.is_enabled() && ui.rect_contains_pointer(area){if let Some(pos)=pos{interaction.marquee=Some(Marquee{start:pos,end:pos,initial:selected.clone(),additive:ctrl,dragged:false});}}
    if let Some(marquee)=interaction.marquee.as_mut(){
        if let Some(pos)=pos{marquee.end=pos;if marquee.start.distance(pos)>4.0{marquee.dragged=true;}}
        if marquee.dragged{
            *selected=if marquee.additive{marquee.initial.clone()}else{BTreeSet::new()};let selection=Rect::from_two_pos(marquee.start,marquee.end);
            selected.extend(regions.iter().filter(|(_,_,rect)|rect.intersects(selection)).map(|(_,hid,_)|*hid));
        }
    }
    let box_drag=interaction.marquee.as_ref().is_some_and(|marquee|marquee.dragged);
    ui.painter().rect_filled(area,9,CARD);ui.painter().rect_stroke(area,9,Stroke::new(1.0_f32,LINE),egui::StrokeKind::Inside);
    let mut iso_enter=Vec::new();let mut enter_hovered=false;
    for (row,hid,rect) in &regions {let rect=*rect;
        let response=ui.interact(rect,ui.id().with(("key",row,hid)),Sense::click_and_drag());
        if !box_drag && (response.clicked() || response.drag_started() && !shift) {
            let multi=ui.input(|i|i.modifiers.shift||i.modifiers.ctrl);
            if !multi {selected.clear();selected.insert(*hid);}else if response.clicked() {if !selected.insert(*hid){selected.remove(hid);}}else{selected.insert(*hid);}
        }
        let chosen=selected.contains(hid);let state=if colors.is_none(){profile.value(*hid)}else{None};let p=ui.painter();
        if *hid==40 && crate::keyboard::current().has_tall_enter(){iso_enter.push(rect);enter_hovered|=response.hovered();response.on_hover_text("Enter");continue;}
        let rgb=colors.and_then(|colors|colors.get(hid)).map(|color|Color32::from_rgb(color[0],color[1],color[2]));
        let foreground=rgb.map(contrast).unwrap_or(if chosen{ORANGE}else{TEXT});
        p.rect_filled(rect,5,rgb.unwrap_or(if chosen{Color32::from_rgb(57,43,34)}else if response.hovered(){Color32::from_rgb(48,53,64)}else{Color32::from_rgb(20,23,29)}));
        p.rect_stroke(rect,5,Stroke::new(if chosen{2.0_f32}else{1.0_f32},if chosen{foreground}else{LINE}),egui::StrokeKind::Inside);
        let label=match hid {42=>"⌫".into(),224|228=>"Ctrl".into(),225|229=>"Shift".into(),44=>String::new(),_=>crate::keyboard::current().keycap(*hid)};
        p.text(rect.min+Vec2::new(7.0,6.0),Align2::LEFT_TOP,label,FontId::proportional((unit*0.25).clamp(11.0,15.0)),foreground);
        if let Some(state)=state {
            if let Some(t)=state.tenths(){p.text(rect.left_bottom()+Vec2::new(7.0,-5.0),Align2::LEFT_BOTTOM,format!("{:.1}",t as f32/10.0),FontId::proportional(10.0),MUTED);}
            if state.rapid(){p.circle_filled(rect.right_bottom()+Vec2::new(-8.0,-7.0),2.3,ORANGE);}
            if state.protection(){p.rect_filled(Rect::from_center_size(rect.right_top()+Vec2::new(-8.0,9.0),Vec2::new(5.0,6.0)),2,BLUE);}
            if state.binding[0]==0x71 {p.circle_filled(rect.right_center()+Vec2::new(-8.0,0.0),2.5,Color32::from_rgb(192,139,255));}
            if state.binding[0]==0x51 && state.binding[1]!=*hid {
                let label=protocol::key_label(state.binding[1]);let target=if label.chars().count()<=4{label}else{"…".into()};
                p.text(rect.center(),Align2::CENTER_CENTER,format!("→ {target}"),FontId::proportional(10.0),BLUE);
            }
        }
        let output=profile.value(*hid).filter(|state|state.binding[0]==0x51).map(|state|crate::i18n::format_text(" · Ausgabe: {}", &[format!("{}", protocol::key_label(state.binding[1]))])).unwrap_or_default();
        response.on_hover_text(crate::i18n::format_text("{}{output} · Klicken zum Auswählen", &[format!("{}", protocol::key_label(*hid)), format!("{}", output)]));
    }
    if iso_enter.len()==2 {
        let top=iso_enter[0];let bottom=iso_enter[1];let p=ui.painter();let chosen=selected.contains(&40);
        let rgb=colors.and_then(|colors|colors.get(&40)).map(|color|Color32::from_rgb(color[0],color[1],color[2]));
        let foreground=rgb.map(contrast).unwrap_or(if chosen{ORANGE}else{TEXT});
        let fill=rgb.unwrap_or(if chosen{Color32::from_rgb(57,43,34)}else if enter_hovered{Color32::from_rgb(48,53,64)}else{Color32::from_rgb(20,23,29)});
        p.rect_filled(Rect::from_min_max(top.min,Pos2::new(top.max.x,bottom.min.y+1.0)),0,fill);p.rect_filled(bottom,0,fill);
        p.add(egui::Shape::closed_line(vec![top.left_top(),top.right_top(),bottom.right_bottom(),bottom.left_bottom(),bottom.left_top(),Pos2::new(top.left(),bottom.top())],Stroke::new(if chosen{2.0_f32}else{1.0_f32},if chosen{foreground}else{LINE})));
        p.text(top.min+Vec2::new(7.0,6.0),Align2::LEFT_TOP,"Enter",FontId::proportional(12.0),foreground);
        if let Some(state)=if colors.is_none(){profile.value(40)}else{None}{
            if let Some(t)=state.tenths(){p.text(bottom.left_bottom()+Vec2::new(7.0,-5.0),Align2::LEFT_BOTTOM,format!("{:.1}",t as f32/10.0),FontId::proportional(10.0),MUTED);}
            if state.rapid(){p.circle_filled(bottom.right_bottom()+Vec2::new(-8.0,-7.0),2.3,ORANGE);}
            if state.protection(){p.rect_filled(Rect::from_center_size(top.right_top()+Vec2::new(-8.0,9.0),Vec2::new(5.0,6.0)),2,BLUE);}
            if state.binding[0]==0x71{p.circle_filled(bottom.right_center()+Vec2::new(-8.0,0.0),2.5,Color32::from_rgb(192,139,255));}
            if state.binding[0]==0x51 && state.binding[1]!=40{p.text(bottom.center(),Align2::CENTER_CENTER,format!("→ {}",protocol::key_label(state.binding[1])),FontId::proportional(10.0),BLUE);}
        }
    }
    if let Some(marquee)=interaction.marquee.as_ref().filter(|marquee|marquee.dragged){let rect=Rect::from_two_pos(marquee.start,marquee.end);ui.painter().rect_filled(rect,2,BLUE.gamma_multiply(0.15));ui.painter().rect_stroke(rect,2,Stroke::new(1.0_f32,BLUE),egui::StrokeKind::Inside);}
    if released{interaction.marquee=None;}
}
fn keyboard_thumbnail(p:&egui::Painter,rect:Rect,name:&str) {
    let width=(rect.width()*0.77).min(390.0);let height=width*0.31;let body=Rect::from_center_size(rect.center(),Vec2::new(width,height));
    p.rect_filled(body,8,Color32::from_rgb(12,14,18));p.rect_stroke(body,8,Stroke::new(1.0_f32,Color32::from_rgb(69,76,88)),egui::StrokeKind::Inside);
    let columns=if name.contains("Mini"){14}else{18};let gap=3.0;let unit=(width-18.0)/columns as f32;let kh=(height-18.0)/5.0;
    for row in 0..5 {for col in 0..columns {
        if row==4 && (4..9).contains(&col){continue;}
        let w=if row==4 && col==3{unit*6.0-gap}else{unit-gap};
        let key=Rect::from_min_size(body.min+Vec2::new(9.0+col as f32*unit,9.0+row as f32*kh),Vec2::new(w,kh-gap));
        p.rect_filled(key,2,Color32::from_rgb(34,39,48));p.rect_stroke(key,2,Stroke::new(0.5_f32,Color32::from_rgb(56,62,73)),egui::StrokeKind::Inside);
        if row<4 {p.line_segment([key.min+Vec2::new(3.0,3.0),key.min+Vec2::new(6.0,3.0)],Stroke::new(1.0_f32,ORANGE.gamma_multiply(0.65)));}
    }}
}
fn event_hid(key:egui::Key)->Option<u8> {
    let name=format!("{key:?}");if name.len()==1 {let c=name.as_bytes()[0];if c.is_ascii_uppercase(){return Some(4+c-b'A');}}
    if let Some(n)=name.strip_prefix("Num").and_then(|x|x.parse::<u8>().ok()){return Some(if n==0{39}else{29+n});}
    if let Some(n)=name.strip_prefix('F').and_then(|x|x.parse::<u8>().ok()){if (1..=12).contains(&n){return Some(57+n);}if (13..=24).contains(&n){return Some(91+n);}}
    match key {egui::Key::Enter=>Some(40),egui::Key::Escape=>Some(41),egui::Key::Backspace=>Some(42),egui::Key::Tab=>Some(43),egui::Key::Space=>Some(44),egui::Key::ArrowRight=>Some(79),egui::Key::ArrowLeft=>Some(80),egui::Key::ArrowDown=>Some(81),egui::Key::ArrowUp=>Some(82),_=>None}
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rgb_selected_b_only_updates_b_after_success_and_preserves_other_key_colors(){
        let (mut app,requests,responses,device)=fixture();app.devices=vec![device];app.busy=false;app.open_device(0);app.selected=BTreeSet::from([5]);app.rgb_color=[0,255,0];app.rgb_brightness=100;
        app.rgb_ui.colors.entry("test".into()).or_default().insert(12,[0,0,255]);
        app.apply_rgb_selection();match requests.try_recv().unwrap(){Request::Rgb{keys,color,..}=>{assert_eq!(keys,vec![5]);assert_eq!(color,[0,255,0]);},_=>panic!("single selected key must use static RGB")}
        assert!(!app.rgb_ui.colors["test"].contains_key(&5));
        responses.send(Response::Rgb(Err("failed".into()))).unwrap();app.poll();assert!(!app.rgb_ui.colors["test"].contains_key(&5));
        app.apply_rgb_selection();requests.try_recv().unwrap();responses.send(Response::Rgb(Ok(()))).unwrap();app.poll();
        assert_eq!(app.rgb_ui.colors["test"][&5],[0,255,0]);assert_eq!(app.rgb_ui.colors["test"][&12],[0,0,255]);assert!(!app.dirty());
    }
    #[test]
    fn animation_preview_keeps_only_latest_frame_and_errors_do_not_complete_profile_jobs(){
        let (mut app,requests,responses,device)=fixture();app.devices=vec![device];app.busy=false;app.open_device(0);app.selected=BTreeSet::from([5]);app.rgb_ui.mode=crate::rgb::Mode::Wave;
        app.apply_rgb_selection();match requests.try_recv().unwrap(){Request::RgbEffect{config,..}=>assert_eq!(config.keys.iter().map(|p|p.hid).collect::<Vec<_>>(),vec![5]),_=>panic!("expected selected effect")}
        responses.send(Response::RgbStarted(Ok(()))).unwrap();app.poll();assert!(app.rgb_ui.active.is_some());
        for n in 0..1000{*app.rgb_ui.preview.lock().unwrap()=Some(("test".into(),vec![(5,[(n%256) as u8,0,0])]));}
        app.poll();assert_eq!(app.rgb_ui.colors["test"][&5],[231,0,0]);assert!(app.rgb_ui.preview.lock().unwrap().is_none());
        app.busy=true;responses.send(Response::RgbError("device removed".into())).unwrap();app.poll();assert!(app.busy);assert!(app.rgb_ui.active.is_none());
    }
    #[test]
    fn lighting_preview_fills_the_entire_b_key_with_exact_color_and_keeps_the_label(){
        let (_,_,_,device)=fixture();let profile=&device.profiles[0];let colors=BTreeMap::from([(5,[0,255,0])]);let mut selected=BTreeSet::from([5]);let mut interaction=KeyboardInteraction::default();
        crate::keyboard::with_layout(crate::keyboard::Layout::GermanIso,||{
            let ctx=egui::Context::default();let output=ctx.run(egui::RawInput{screen_rect:Some(Rect::from_min_size(Pos2::ZERO,Vec2::new(1100.0,850.0))),..Default::default()},|ctx|{egui::CentralPanel::default().show(ctx,|ui|keyboard(ui,profile,&mut selected,Some(&colors),&mut interaction));});
            let green:Vec<_>=output.shapes.iter().filter_map(|shape|match &shape.shape{egui::Shape::Rect(rect) if rect.fill==Color32::from_rgb(0,255,0)=>Some(rect.rect),_=>None}).collect();
            assert_eq!(green.len(),1);assert!(green[0].width()>40.0 && green[0].height()>30.0);
            assert!(output.shapes.iter().any(|shape|matches!(&shape.shape,egui::Shape::Text(text) if text.galley.job.text=="B" && green[0].contains(text.pos))));
        });
    }
    #[test]
    fn shift_drag_selects_a_rectangle_and_shift_click_still_adds_one_key(){
        crate::keyboard::with_layout(crate::keyboard::Layout::GermanIso,||{
            let (_,_,_,device)=fixture();let profile=&device.profiles[0];let mut selected=BTreeSet::from([5]);let mut interaction=KeyboardInteraction::default();let ctx=egui::Context::default();
            fn draw(ctx:&egui::Context,profile:&Profile,selected:&mut BTreeSet<u8>,interaction:&mut KeyboardInteraction,events:Vec<egui::Event>)->Vec<(usize,u8,Rect)>{
                let mut regions=Vec::new();let _=ctx.run(egui::RawInput{screen_rect:Some(Rect::from_min_size(Pos2::ZERO,Vec2::new(1100.0,850.0))),modifiers:egui::Modifiers{shift:true,..Default::default()},events,..Default::default()},|ctx|{egui::CentralPanel::default().show(ctx,|ui|{let width=ui.available_width().min(1050.0);let unit=(width-18.0)/15.0;regions=key_regions(Rect::from_min_size(ui.cursor().min,Vec2::new(width,unit*0.78*5.0+18.0)),unit,unit*0.78);keyboard(ui,profile,selected,None,interaction);});});regions
            }
            draw(&ctx,profile,&mut selected,&mut interaction,vec![]);let regions=draw(&ctx,profile,&mut selected,&mut interaction,vec![]);
            let a=regions.iter().find(|(_,hid,_)|*hid==4).unwrap().2;let d=regions.iter().find(|(_,hid,_)|*hid==7).unwrap().2;let b=regions.iter().find(|(_,hid,_)|*hid==5).unwrap().2;
            let start=a.min+Vec2::splat(1.0);let end=d.max-Vec2::splat(1.0);let mods=egui::Modifiers{shift:true,..Default::default()};
            draw(&ctx,profile,&mut selected,&mut interaction,vec![egui::Event::PointerMoved(start),egui::Event::PointerButton{pos:start,button:egui::PointerButton::Primary,pressed:true,modifiers:mods}]);
            draw(&ctx,profile,&mut selected,&mut interaction,vec![egui::Event::PointerMoved(end)]);
            draw(&ctx,profile,&mut selected,&mut interaction,vec![egui::Event::PointerButton{pos:end,button:egui::PointerButton::Primary,pressed:false,modifiers:mods}]);
            assert_eq!(selected,BTreeSet::from([4,22,7]));assert!(interaction.marquee.is_none());
            for pressed in [true,false]{draw(&ctx,profile,&mut selected,&mut interaction,vec![egui::Event::PointerMoved(b.center()),egui::Event::PointerButton{pos:b.center(),button:egui::PointerButton::Primary,pressed,modifiers:mods}]);}
            assert_eq!(selected,BTreeSet::from([4,22,7,5]));
        });
    }
    #[test]
    fn keep_editing_button_cancels_the_warning_without_losing_the_profile(){
        let (mut app,requests,_,device)=fixture();app.devices=vec![device];app.busy=false;app.open_device(0);app.profile.as_mut().unwrap().set_binding(&[5],12);
        let before=app.profile.clone();app.load_slot(0,1);let ctx=egui::Context::default();let mut position=None;
        for _ in 0..2{
            let output=ctx.run(egui::RawInput{screen_rect:Some(Rect::from_min_size(Pos2::ZERO,Vec2::new(1100.0,850.0))),..Default::default()},|ctx|app.unsaved_panel(ctx));
            for shape in output.shapes{if let egui::Shape::Text(text)=shape.shape{if text.galley.job.text=="Weiter bearbeiten"{position=Some(text.pos+text.galley.rect.center().to_vec2());}}}
        }
        let pos=position.expect("keep editing button is visible");
        for pressed in [true,false]{let _=ctx.run(egui::RawInput{screen_rect:Some(Rect::from_min_size(Pos2::ZERO,Vec2::new(1100.0,850.0))),events:vec![egui::Event::PointerMoved(pos),egui::Event::PointerButton{pos,button:egui::PointerButton::Primary,pressed,modifiers:egui::Modifiers::default()}],..Default::default()},|ctx|app.unsaved_panel(ctx));}
        assert!(app.safety.pending.is_none());assert!(app.profile==before);assert!(app.dirty());assert!(requests.try_recv().is_err());
    }
    #[test]
    fn dirty_profile_switch_waits_for_confirmation_and_cancel_preserves_draft(){
        let (mut app,requests,responses,device)=fixture();app.devices=vec![device];app.busy=false;app.open_device(0);assert!(!app.dirty());
        app.profile.as_mut().unwrap().set_binding(&[5],12);let changed=app.profile.clone();
        app.load_slot(0,1);assert!(app.safety.pending.is_some());assert!(requests.try_recv().is_err());assert!(app.profile==changed);
        app.safety.pending=None;assert!(app.dirty());assert!(app.profile==changed);
        app.load_slot(0,1);app.resolve_pending(false);
        assert!(matches!(requests.try_recv().unwrap(),Request::Load{slot:1,..}));assert!(app.profile.is_none());
        responses.send(Response::Loaded(Err("read failed".into()))).unwrap();app.poll();assert!(app.profile.is_none());
    }
    #[test]
    fn resetting_a_draft_does_not_write_or_reset_to_the_destination_slot(){
        let (mut app,requests,_,device)=fixture();app.devices=vec![device];app.busy=false;app.open_device(0);let original=app.profile.clone();
        app.profile.as_mut().unwrap().set_binding(&[5],12);app.profile_name="Changed".into();app.live_preview=true;
        app.slot=3;app.baseline=Some(app.devices[0].profiles[3].clone());
        app.request_action(PendingAction::Reset);assert!(app.safety.pending.is_some());app.resolve_pending(false);
        assert!(app.profile==original);assert_eq!(app.profile_name,"Profil 1");assert!(!app.dirty());assert!(!app.live_preview);
        assert_eq!(app.baseline.as_ref().unwrap().name(),"Profil 4");assert!(requests.try_recv().is_err());
    }
    #[test]
    fn backup_preview_never_writes_and_explicit_store_uses_selected_target_snapshot(){
        let (mut app,requests,responses,device)=fixture();let expected=device.profiles[3].clone();let mut backup=device.profiles[0].clone();backup.set_binding(&[5],12);
        app.devices=vec![device];app.busy=false;app.open_device(0);app.live_preview=true;
        app.request_action(PendingAction::Backup(0,3,backup.clone()));
        assert!(app.profile.as_ref()==Some(&backup));assert_eq!(app.slot,3);assert!(app.dirty());assert!(!app.live_preview);assert!(requests.try_recv().is_err());
        app.store();match requests.try_recv().unwrap(){Request::Store{slot,profile,expected:before,..}=>{assert_eq!(slot,3);assert!(profile==backup);assert!(before==expected);},_=>panic!("expected explicit profile store")}
        responses.send(Response::Stored(Err("write failed".into()))).unwrap();app.poll();assert!(app.dirty());
        responses.send(Response::Stored(Ok((3,backup)))).unwrap();app.poll();assert!(!app.dirty());assert!(app.profile==app.baseline);
    }
    #[test]
    fn quitting_a_dirty_profile_waits_for_the_user(){
        let (mut app,requests,_,device)=fixture();app.devices=vec![device];app.busy=false;app.open_device(0);app.profile_name="Edited".into();
        app.request_action(PendingAction::Quit);assert!(!app.allow_exit);assert!(!app.quit_pending);assert!(app.safety.pending.is_some());
        app.resolve_pending(false);assert!(app.allow_exit);assert!(app.quit_pending);assert!(requests.try_recv().is_err());
    }
    #[test]
    fn failed_local_save_keeps_the_dialog_and_successful_save_preserves_onboard_state(){
        let folder=std::env::temp_dir().join(format!("apexcontrol-draft-save-test-{}",std::process::id()));std::fs::create_dir_all(&folder).unwrap();
        backend::with_data_dir(folder.clone(),||{
            let (mut app,requests,_,device)=fixture();app.devices=vec![device.clone()];app.busy=false;app.open_device(0);let baseline=app.baseline.clone();
            app.profile.as_mut().unwrap().set_binding(&[5],12);app.profile_name="Local draft".into();app.request_action(PendingAction::Scan);
            std::fs::write(folder.join("profiles"),b"blocks saving").unwrap();app.resolve_pending(true);
            assert!(app.error);assert!(app.dirty());assert!(app.safety.pending.is_some());assert!(requests.try_recv().is_err());
            std::fs::remove_file(folder.join("profiles")).unwrap();app.resolve_pending(true);
            assert!(!app.dirty());assert!(app.safety.pending.is_none());assert!(app.baseline==baseline);assert!(app.profile!=app.baseline);
            assert!(matches!(requests.try_recv().unwrap(),Request::Scan));
            let session:Session=serde_json::from_slice(&serde_json::to_vec(app.session.as_ref().unwrap()).unwrap()).unwrap();
            let (mut restarted,_,_,_)=fixture();restarted.devices=vec![device];restarted.busy=false;restarted.session=Some(session);restarted.open_device(0);
            assert!(!restarted.dirty());assert_eq!(restarted.profile_name,"Local draft");assert_eq!(backend::saved_profiles().len(),1);
        });assert!(folder.starts_with(std::env::temp_dir()));std::fs::remove_dir_all(folder).unwrap();
    }
    #[test]
    fn one_frame_of_macro_input_cannot_overflow_or_leave_pressed_keys(){
        let (mut app,_,_,device)=fixture();app.recording=true;
        let ctx=egui::Context::default();
        let events=(0..4000).map(|i|egui::Event::Key{key:egui::Key::B,physical_key:Some(egui::Key::B),pressed:i%2==0,repeat:false,modifiers:egui::Modifiers::default()}).collect();
        let _=ctx.run(egui::RawInput{events,..Default::default()},|ctx|app.record_events(ctx));
        assert!(!app.recording);assert!(app.held.is_empty());assert!(app.recorded.len()<=1000);
        assert!(!app.recorded.is_empty());
        let mut profile=device.profiles[0].clone();
        profile.assign_first_macro(&[5],&app.recorded).unwrap();
        assert_eq!(profile.macros()[0].events,app.recorded);
    }
    #[test]
    fn first_run_focuses_english_language_question_without_a_settings_arrow(){
        crate::i18n::with_language(crate::i18n::Language::German,||{
            let (mut app,_,_,_)=fixture();app.settings.language_setup_done=false;
            let ctx=egui::Context::default();let mut labels=BTreeSet::new();let mut arrow=false;
            for _ in 0..2{
                let output=ctx.run(egui::RawInput{screen_rect:Some(Rect::from_min_size(Pos2::ZERO,Vec2::new(1100.0,740.0))),..Default::default()},|ctx|{app.header(ctx);app.language_intro(ctx);});
                for shape in output.shapes{match shape.shape{
                    egui::Shape::Text(text)=>{labels.insert(text.galley.job.text.clone());},
                    egui::Shape::LineSegment{points,stroke} if stroke.color==ORANGE && points[0].distance(points[1])>100.0=>arrow=true,_=>{},
                }}
            }
            for expected in ["Welcome to ApexControl","Would you like to use ApexControl in English or German?","English","Deutsch"]{assert!(labels.contains(expected),"missing welcome text: {expected}");}
            assert!(!arrow);assert!(app.intro_focus_set);assert!(ctx.memory(|memory|memory.focused().is_some()));assert!(!app.settings.language_setup_done);
            app.settings.language_setup_done=true;
            let output=ctx.run(egui::RawInput::default(),|ctx|app.language_intro(ctx));
            assert!(!output.shapes.iter().any(|shape|matches!(&shape.shape,egui::Shape::Text(text) if text.galley.job.text=="Welcome to ApexControl")));
        });
    }
    #[test]
    fn language_buttons_lead_to_a_localized_layout_dropdown_before_saving(){
        fn render(app:&mut ApexApp,ctx:&egui::Context,events:Vec<egui::Event>)->egui::FullOutput{
            ctx.run(egui::RawInput{screen_rect:Some(Rect::from_min_size(Pos2::ZERO,Vec2::new(1100.0,740.0))),events,..Default::default()},|ctx|app.language_intro(ctx))
        }
        fn text_position(output:egui::FullOutput,label:&str)->Pos2{
            output.shapes.into_iter().find_map(|shape|match shape.shape{egui::Shape::Text(text) if text.galley.job.text==label=>Some(text.pos+text.galley.rect.center().to_vec2()),_=>None}).unwrap_or_else(||panic!("missing: {label}"))
        }
        fn click(app:&mut ApexApp,ctx:&egui::Context,pos:Pos2){for pressed in [true,false]{render(app,ctx,vec![egui::Event::PointerMoved(pos),egui::Event::PointerButton{pos,button:egui::PointerButton::Primary,pressed,modifiers:egui::Modifiers::default()}]);}}
        for (button,language,title,prompt,usa) in [
            ("English",crate::i18n::Language::English,"Choose your keyboard layout","Choose a layout …","US (ANSI)"),
            ("Deutsch",crate::i18n::Language::German,"Tastaturlayout wählen","Layout auswählen …","USA (ANSI)"),
        ]{
            crate::i18n::with_language(crate::i18n::Language::English,||{
                let (mut app,_,_,_)=fixture();app.settings.language_setup_done=false;app.settings.layout_setup_done=false;
                let ctx=egui::Context::default();render(&mut app,&ctx,vec![]);
                let pos=text_position(render(&mut app,&ctx,vec![]),button);click(&mut app,&ctx,pos);
                assert_eq!(app.intro_language,Some(language));assert!(!app.settings.setup_complete());
                render(&mut app,&ctx,vec![]);let output=render(&mut app,&ctx,vec![]);
                assert!(output.shapes.iter().any(|shape|matches!(&shape.shape,egui::Shape::Text(text) if text.galley.job.text==title)));
                let pos=text_position(output,prompt);click(&mut app,&ctx,pos);
                let pos=text_position(render(&mut app,&ctx,vec![]),usa);click(&mut app,&ctx,pos);
                assert_eq!(app.intro_layout,Some(crate::keyboard::Layout::UsAnsi));
                assert!(!app.settings.setup_complete());assert!(app.settings.keyboard_layout.is_none());
            });
        }
    }
    #[test]
    fn english_interface_translates_settings_bindings_and_macros() {
        crate::i18n::with_language(crate::i18n::Language::English,||{
            let (mut app,_,_,mut device)=fixture();device.profiles[0].set_name("Mein Profil Ä");
            app.devices=vec![device];app.busy=false;app.open_device(0);app.settings_open=true;
            let ctx=egui::Context::default();let mut labels=std::collections::BTreeSet::new();
            for tab in [Tab::Bindings,Tab::Macros] {
                app.tab=tab;
                for _ in 0..2 {
                    let output=ctx.run(egui::RawInput{screen_rect:Some(Rect::from_min_size(Pos2::ZERO,Vec2::new(1320.0,1000.0))),..Default::default()},|ctx|{
                        app.header(ctx);app.footer(ctx);app.sidebar(ctx,0);
                        egui::CentralPanel::default().show(ctx,|ui|{egui::ScrollArea::vertical().show(ui,|ui|app.editor(ui,0));});
                        app.settings_panel(ctx);
                    });
                    for shape in output.shapes {if let egui::Shape::Text(text)=shape.shape{labels.insert(text.galley.job.text.clone());}}
                }
            }
            for expected in ["Remap a key","Assign","Key bindings","Macro editor","Function keys","Add key","Start with Windows","Minimize to the system tray when closed","Save to keyboard"] {
                assert!(labels.contains(expected),"missing English UI label: {expected}");
            }
            for german in ["Eine Taste umbelegen","Zuweisen","Tastenbelegung","Makro-Editor","Taste hinzufügen","Mit Windows starten"]{assert!(!labels.contains(german),"untranslated label: {german}");}
            assert_eq!(app.profile_name,"Mein Profil Ä");
        });
    }
    #[test]
    fn b_to_i_selection_survives_frames_and_assigns_only_b() {
        fn render(app:&mut ApexApp,ctx:&egui::Context,events:Vec<egui::Event>)->egui::FullOutput {
            ctx.run(egui::RawInput{screen_rect:Some(Rect::from_min_size(Pos2::ZERO,Vec2::new(800.0,600.0))),events,..Default::default()},|ctx|{
                egui::CentralPanel::default().show(ctx,|ui|app.binding_controls(ui));
            })
        }
        let (mut app,requests,_,device)=fixture();let mut profile=device.profiles[0].clone();
        profile.set_binding(&[5],5);profile.set_binding(&[12],12);
        app.profile=Some(profile);app.busy=false;
        let ctx=egui::Context::default();render(&mut app,&ctx,vec![]);
        app.binding_picker.as_mut().unwrap().target=12; // Choose I for physical B.
        let output=render(&mut app,&ctx,vec![]);
        assert_eq!(app.binding_picker.as_ref().unwrap().target,12);
        let mut button=None;let mut preview=false;
        for shape in output.shapes {if let egui::Shape::Text(text)=shape.shape {
            if text.galley.job.text=="B → I" {preview=true;}
            if text.galley.job.text=="Zuweisen" {button=Some(text.pos+text.galley.rect.center().to_vec2());}
        }}
        assert!(preview);let pos=button.expect("assignment button rendered");
        for pressed in [true,false] {render(&mut app,&ctx,vec![egui::Event::PointerMoved(pos),egui::Event::PointerButton{pos,button:egui::PointerButton::Primary,pressed,modifiers:egui::Modifiers::default()}]);}
        let profile=app.profile.as_ref().unwrap();
        assert_eq!(profile.value(5).unwrap().binding,[0x51,12,0,0,0]);
        assert_eq!(profile.value(12).unwrap().binding,[0x51,12,0,0,0]);
        Profile::parse(profile.bytes.clone()).unwrap();
        assert!(matches!(requests.try_recv(),Err(mpsc::TryRecvError::Empty)),"assign only edits the draft until explicit save");
        app.selected=BTreeSet::from([12]);render(&mut app,&ctx,vec![]);
        assert_eq!(app.binding_picker.as_ref().unwrap().keys,vec![12]);
    }
    fn fixture()->(ApexApp,mpsc::Receiver<Request>,mpsc::Sender<Response>,Device) {
        let (tx,requests)=mpsc::channel();let (responses,rx)=mpsc::channel();
        let profiles=(0..5).map(|slot|{let mut bytes=vec![0;protocol::PROFILE_LEN];bytes[0]=3;bytes[780]=6;
            let mut profile=Profile{bytes};profile.set_name(&format!("Profil {}",slot+1));profile}).collect();
        let device=Device{id:"test".into(),name:"Apex Pro Mini".into(),pid:protocol::PID_MINI,path:std::ffi::CString::new("test").unwrap(),supported:true,profiles,firmware:"1.19.7".into(),error:None,region_id:None,layout_id:None,detected_layout:None};
        let app=ApexApp{_instance:None,safety:EditorSafety::default(),backups:BackupBrowser::default(),rgb_ui:RgbUi::default(),keyboard_interaction:KeyboardInteraction::default(),tx,rx,devices:vec![],saved:vec![],device:None,editor:false,busy:true,status:String::new(),error:false,
            slot:2,profile:None,baseline:None,selected:BTreeSet::from([5]),tab:Tab::Actuation,profile_name:String::new(),live_preview:false,
            card_choices:vec![],recording:false,recorded:vec![],held:BTreeSet::new(),last_event:None,macro_name:String::new(),library:vec![],macro_choice:0,
            rgb_color:[255,143,79],rgb_brightness:65,session:None,macro_category:KeyCategory::Function,macro_key:58,macro_action:0,macro_delay:100,binding_picker:None,settings:Settings{language_setup_done:true,layout_setup_done:true,..Default::default()},settings_open:false,updates_open:false,scan_failed:false,autostart:false,autostart_error:None,autostart_poll:None,desktop:None,desktop_error:None,start_hidden:None,quit_pending:false,allow_exit:false,intro_language:None,intro_layout:None,intro_focus_set:false,intro_error:None};
        (app,requests,responses,device)
    }
    #[test]
    fn startup_stays_on_overview_and_explicit_open_restores_the_last_profile() {
        let (mut app,requests,responses,mut device)=fixture();
        let mut draft=device.profiles[3].clone();draft.set_name("Lokaler Entwurf");
        app.session=Some(Session{device_id:device.id.clone(),slot:3,draft:Some(draft.clone()),name:"Mein Profil".into(),checkpoint:None});
        device.profiles[3].set_name("Frisch vom Gerät");let expected=device.profiles[3].clone();
        responses.send(Response::Scanned(Ok(vec![device]))).unwrap();app.poll();
        assert!(!app.editor);assert!(app.profile.is_none());assert_eq!(app.card_choices,vec![3]);
        assert!(matches!(requests.try_recv(),Err(mpsc::TryRecvError::Empty)));
        app.open_device(0);
        assert!(app.editor);assert_eq!(app.slot,3);assert_eq!(app.profile_name,"Mein Profil");
        assert!(app.profile.as_ref()==Some(&draft));assert!(app.baseline.as_ref()==Some(&expected));
        assert!(matches!(requests.try_recv(),Err(mpsc::TryRecvError::Empty)));
    }
    #[test]
    fn first_start_stays_on_overview_until_the_device_is_opened() {
        let (mut app,requests,responses,device)=fixture();
        responses.send(Response::Scanned(Ok(vec![device]))).unwrap();app.poll();
        assert!(!app.editor);assert!(app.device.is_none());assert!(app.profile.is_none());
        app.open_device(0);
        assert!(app.editor);assert_eq!(app.slot,0);assert!(app.status.contains("unbekannt"));
        assert!(matches!(requests.try_recv(),Err(mpsc::TryRecvError::Empty)));
    }
    #[test]
    fn manual_macro_controls_fit_the_editor() {
        let (mut app,_,_,device)=fixture();app.devices=vec![device];app.busy=false;app.open_device(0);app.tab=Tab::Macros;
        let ctx=egui::Context::default();
        let output=ctx.run(egui::RawInput{screen_rect:Some(Rect::from_min_size(Pos2::ZERO,Vec2::new(1100.0,850.0))),..Default::default()},|ctx|{
            app.header(ctx);app.footer(ctx);app.sidebar(ctx,0);
            egui::CentralPanel::default().show(ctx,|ui|{egui::ScrollArea::vertical().show(ui,|ui|app.editor(ui,0));});
        });
        let mut found=false;
        for shape in output.shapes {if let egui::Shape::Text(text)=shape.shape {if text.galley.job.text=="Taste hinzufügen" {
            found=true;assert_eq!(text.galley.rows.len(),1);assert!(text.pos.x+text.galley.rect.max.x<=1100.0);
        }}}
        assert!(found,"manual insertion button must be rendered");
    }
    #[test]
    fn settings_show_all_requested_choices_without_enabling_them() {
        let (mut app,_,_,_)=fixture();app.settings_open=true;
        let ctx=egui::Context::default();
        let mut labels=std::collections::BTreeSet::new();
        for _ in 0..2 {
            let output=ctx.run(egui::RawInput{screen_rect:Some(Rect::from_min_size(Pos2::ZERO,Vec2::new(1100.0,740.0))),..Default::default()},|ctx|app.settings_panel(ctx));
            for shape in output.shapes {if let egui::Shape::Text(text)=shape.shape {labels.insert(text.galley.job.text.clone());}}
        }
        for label in ["Mit Windows starten","Beim Windows-Autostart im Hintergrund starten","Beim Schließen in den Infobereich minimieren"] {assert!(labels.contains(label),"missing preference: {label}");}
        assert!(!app.autostart);assert!(!app.settings.minimize_on_close);assert!(!app.settings.autostart_minimized);
    }
    #[test]
    fn background_start_hides_after_the_frameworks_initial_reveal() {
        let (mut app,_,_,_)=fixture();app.start_hidden=Some(1);
        let ctx=egui::Context::default();
        let first=ctx.run(egui::RawInput::default(),|ctx|app.desktop_events(ctx));
        assert!(!first.viewport_output[&egui::ViewportId::ROOT].commands.contains(&egui::ViewportCommand::Visible(false)));
        let second=ctx.run(egui::RawInput::default(),|ctx|app.desktop_events(ctx));
        assert!(second.viewport_output[&egui::ViewportId::ROOT].commands.contains(&egui::ViewportCommand::Visible(false)));
        assert!(app.start_hidden.is_none());
    }
    #[test]
    fn device_card_buttons_stay_on_one_line() {
        // Render synthetic devices only: no HID access and no personal profiles.
        for width in [1100.0,1320.0] {
            let (tx,_)=mpsc::channel();let (_,rx)=mpsc::channel();
            let device=Device{id:"test".into(),name:"Apex Pro Mini".into(),pid:protocol::PID_MINI,
                path:std::ffi::CString::new("test").unwrap(),supported:true,
                profiles:(0..5).map(|_|Profile{bytes:vec![0;protocol::PROFILE_LEN]}).collect(),firmware:"1.19.7".into(),error:None,region_id:None,layout_id:None,detected_layout:None};
            let mut app=ApexApp{_instance:None,safety:EditorSafety::default(),backups:BackupBrowser::default(),rgb_ui:RgbUi::default(),keyboard_interaction:KeyboardInteraction::default(),tx,rx,devices:vec![device.clone(),device],saved:vec![],device:None,
                editor:false,busy:false,status:String::new(),error:false,slot:2,profile:None,baseline:None,
                selected:BTreeSet::new(),tab:Tab::Actuation,profile_name:String::new(),live_preview:false,
                card_choices:vec![2,2],recording:false,recorded:vec![],held:BTreeSet::new(),last_event:None,
                macro_name:String::new(),library:vec![],macro_choice:0,rgb_color:[255,143,79],rgb_brightness:65,
                session:None,macro_category:KeyCategory::Function,macro_key:58,macro_action:0,macro_delay:100,binding_picker:None,settings:Settings{language_setup_done:true,layout_setup_done:true,..Default::default()},settings_open:false,updates_open:false,scan_failed:false,autostart:false,autostart_error:None,autostart_poll:None,desktop:None,desktop_error:None,start_hidden:None,quit_pending:false,allow_exit:false,intro_language:None,intro_layout:None,intro_focus_set:false,intro_error:None};
            let ctx=egui::Context::default();
            let output=ctx.run(egui::RawInput{screen_rect:Some(Rect::from_min_size(Pos2::ZERO,Vec2::new(width,850.0))),..Default::default()},|ctx|{
                egui::CentralPanel::default().show(ctx,|ui|app.home(ui));
            });
            let mut buttons=0;
            for shape in output.shapes {
                if let egui::Shape::Text(text)=shape.shape {
                    if ["Profil laden","Öffnen"].contains(&text.galley.job.text.as_str()) {
                        buttons+=1;
                        assert_eq!(text.galley.rows.len(),1,"button wrapped at width {width}");
                        assert!(text.pos.x>=0.0 && text.pos.x+text.galley.rect.max.x<=width,"button outside window at width {width}");
                    }
                }
            }
            assert_eq!(buttons,4,"two complete device cards must be visible");
        }
    }
}








