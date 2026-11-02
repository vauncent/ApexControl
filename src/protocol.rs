use serde::{Deserialize, Serialize};

pub const PID_MINI: u16 = 0x161e;
pub const PROFILE_LEN: usize = 11500;
pub const CRC_OFFSET: usize = 11296;
pub const KEY_ORDER: [u8; 68] = [41,30,31,32,33,34,35,36,37,38,39,45,46,137,43,20,26,8,21,23,28,24,12,18,19,47,48,49,57,4,22,7,9,10,11,13,14,15,51,52,50,40,225,100,29,27,6,25,5,17,16,54,55,56,135,229,224,227,226,139,44,138,136,230,231,240,228,42];
pub const HIGH: usize = 784;
pub const LOW: usize = 854;
pub const MODE: usize = 1064;
pub const SENS: usize = 1134;
pub const MACROS: usize = 1680;
// The 16 bytes before the CRC are separate profile metadata, not macro storage.
pub const MACRO_END: usize = MACROS + 9600;

#[derive(Clone, Deserialize)]
pub struct ActuationPoint { pub mm: f32, pub low: u8, pub high: u8 }
pub fn actuation_points() -> Vec<ActuationPoint> { serde_json::from_str(include_str!("actuation.json")).expect("embedded actuation table") }
pub fn key_index(hid: u8) -> Option<usize> { KEY_ORDER.iter().position(|k| *k == hid) }
pub fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xffff_ffffu32;
    for word in bytes.chunks_exact(4) {
        crc ^= u32::from_le_bytes(word.try_into().unwrap());
        for _ in 0..32 { crc = if crc & 0x8000_0000 != 0 { (crc << 1) ^ 0x04c1_1db7 } else { crc << 1 }; }
    }
    crc
}

#[derive(Clone, Serialize, PartialEq)]
pub struct Profile { pub bytes: Vec<u8> }
// JSON must pass the same format/length/CRC checks as a device profile. A
// derived deserializer would allow construction of an unchecked Profile.
impl<'de> Deserialize<'de> for Profile {
    fn deserialize<D:serde::Deserializer<'de>>(deserializer:D)->Result<Self,D::Error>{
        #[derive(Deserialize)] struct Data{bytes:Vec<u8>}
        let data=Data::deserialize(deserializer)?;
        Self::parse(data.bytes).map_err(serde::de::Error::custom)
    }
}
impl Profile {
    pub fn parse(bytes: Vec<u8>) -> Result<Self, String> {
        if bytes.len() != PROFILE_LEN { return Err(crate::i18n::text("Unvollständiges Tastaturprofil").into()); }
        if u32::from_le_bytes(bytes[0..4].try_into().unwrap()) != 3 || u32::from_le_bytes(bytes[780..784].try_into().unwrap()) != 6 {
            return Err(crate::i18n::text("Dieses Profilformat wird noch nicht unterstützt").into());
        }
        let stored = u32::from_le_bytes(bytes[CRC_OFFSET..CRC_OFFSET+4].try_into().unwrap());
        if crc32(&bytes[..CRC_OFFSET]) != stored { return Err(crate::i18n::text("Die Prüfsumme des Profils stimmt nicht").into()); }
        Ok(Self { bytes })
    }
    pub fn name(&self) -> String { String::from_utf8_lossy(self.bytes[4..36].split(|b| *b == 0).next().unwrap_or_default()).into_owned() }
    pub fn set_name(&mut self, name: &str) {
        self.bytes[4..36].fill(0);
        let mut length = name.len().min(31);
        while !name.is_char_boundary(length) { length -= 1; }
        self.bytes[4..4+length].copy_from_slice(&name.as_bytes()[..length]);
        self.seal();
    }
    pub fn seal(&mut self) { let crc = crc32(&self.bytes[..CRC_OFFSET]); self.bytes[CRC_OFFSET..CRC_OFFSET+4].copy_from_slice(&crc.to_le_bytes()); }
    pub fn value(&self, hid: u8) -> Option<KeyState> {
        key_index(hid).map(|i| KeyState { hid, high: self.bytes[HIGH+i], low: self.bytes[LOW+i], mode: self.bytes[MODE+i], sensitivity: self.bytes[SENS+i], binding: self.bytes[80+5*i..85+5*i].try_into().unwrap() })
    }
    pub fn set_actuation(&mut self, keys: &[u8], tenths: u8) -> Result<(), String> {
        let points = actuation_points();
        let point = points.get(tenths.checked_sub(1).ok_or(crate::i18n::text("Ungültiger Auslösepunkt"))? as usize).ok_or(crate::i18n::text("Ungültiger Auslösepunkt"))?;
        for key in keys { if let Some(i) = key_index(*key) { self.bytes[HIGH+i] = point.high; self.bytes[LOW+i] = point.low; } }
        self.seal(); Ok(())
    }
    pub fn set_rapid(&mut self, keys: &[u8], enabled: bool) {
        for key in keys { if let Some(i) = key_index(*key) {
            self.bytes[MODE+i] = match (self.bytes[MODE+i], enabled) { (0,true)=>2,(3,true)=>4,(2,false)=>0,(4,false)=>3,(other,_)=>other };
        } } self.seal();
    }
    pub fn set_protection(&mut self, keys: &[u8], enabled: bool) {
        for key in keys { if let Some(i) = key_index(*key) {
            self.bytes[MODE+i] = match (self.bytes[MODE+i], enabled) { (0,true)=>3,(2,true)=>4,(3,false)=>0,(4,false)=>2,(other,_)=>other };
        } } self.seal();
    }
    pub fn set_sensitivity(&mut self, keys: &[u8], tenths: u8) {
        for key in keys { if let Some(i) = key_index(*key) { self.bytes[SENS+i] = tenths.clamp(1,40); } } self.seal();
    }
    pub fn set_binding(&mut self, keys: &[u8], target: u8) {
        for key in keys { if let Some(i) = key_index(*key) {
            self.bytes[80+5*i..85+5*i].copy_from_slice(&[0x51,target,0,0,0]);
        } } self.seal();
    }
    pub fn reset_binding(&mut self, keys: &[u8]) { for key in keys { self.set_binding(&[*key], *key); } }
    pub fn live_report(&self, sub: u8) -> Result<Vec<u8>, String> {
        if ![0x61,0x62,0x65].contains(&sub) { return Err(crate::i18n::text("Unbekannter Live-Befehl").into()); }
        let mut usages = KEY_ORDER.to_vec(); usages.sort();
        let mut result = vec![0,0x38,sub,68];
        if sub == 0x61 { result.push(0); }
        for hid in usages {
            let value = self.value(hid).unwrap(); result.push(hid);
            match sub { 0x61=>result.extend([value.high,value.low]),0x62=>result.push(value.mode),_=>result.push(value.sensitivity) }
        }
        result.resize(513,0); Ok(result)
    }
    pub fn macros(&self) -> Vec<MacroRecord> {
        let mut at = MACROS; let mut result = Vec::new();
        while at + 8 <= MACRO_END {
            if self.bytes[at] == 0 { at += 1; continue; }
            if self.bytes[at] != 0x77 { break; }
            let count = u16::from_le_bytes([self.bytes[at+2], self.bytes[at+3]]) as usize;
            if count == 0 || count > 2000 || at+8+count*4 > MACRO_END { break; }
            let mut events = Vec::new(); let mut valid = true;
            for chunk in self.bytes[at+8..at+8+count*4].chunks_exact(4) {
                match chunk {
                    [2,key,1,state] if *state <= 1 => events.push(MacroEvent::Key { hid:*key, down:*state==1 }),
                    [4,0,lo,hi] => events.push(MacroEvent::Delay { ms:u16::from_le_bytes([*lo,*hi]) }),
                    _ => { valid = false; break; }
                }
            }
            if !valid { break; }
            result.push(MacroRecord { name:crate::i18n::format_text("Onboard-Makro {}", &[format!("{}", result.len()+1)]), events });
            at += 8+count*4;
        } result
    }
    pub fn assign_first_macro(&mut self, keys: &[u8], events: &[MacroEvent]) -> Result<(), String> {
        // Do not silently destroy another onboard macro or another binding.
        if self.bytes[MACROS..MACRO_END].iter().any(|b| *b != 0) { return Err(crate::i18n::text("Dieses Profil enthält bereits Makrodaten. Bitte ein Profil ohne Makros verwenden.").into()); }
        if events.is_empty() || events.len() > 1000 { return Err(crate::i18n::text("Das Makro ist leer oder zu lang").into()); }
        let mut held = std::collections::BTreeSet::new();
        for event in events { if let MacroEvent::Key { hid,down } = event { if *down { held.insert(*hid); } else { held.remove(hid); } } }
        if !held.is_empty() { return Err(crate::i18n::text("Das Makro enthält nicht losgelassene Tasten").into()); }
        let mut record = vec![0x77,1]; record.extend((events.len() as u16).to_le_bytes()); record.extend([1,1,15,0]);
        for event in events { match event { MacroEvent::Key {hid,down}=>record.extend([2,*hid,1,u8::from(*down)]), MacroEvent::Delay {ms}=>{record.extend([4,0]);record.extend(ms.to_le_bytes());} } }
        if MACROS+record.len() > MACRO_END { return Err(crate::i18n::text("Das Makro passt nicht in das Profil").into()); }
        self.bytes[MACROS..MACROS+record.len()].copy_from_slice(&record);
        for key in keys { if let Some(i) = key_index(*key) { self.bytes[80+5*i..85+5*i].copy_from_slice(&[0x71,0,0,0,0]); } }
        self.seal(); Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct KeyState { pub hid:u8, pub high:u8, pub low:u8, pub mode:u8, pub sensitivity:u8, pub binding:[u8;5] }
impl KeyState {
    pub fn rapid(&self) -> bool { [2,4].contains(&self.mode) }
    pub fn protection(&self) -> bool { [3,4].contains(&self.mode) }
    pub fn tenths(&self) -> Option<u8> { actuation_points().iter().position(|p| p.high==self.high && p.low==self.low).map(|i| i as u8+1) }
}
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub enum MacroEvent { Key { hid:u8, down:bool }, Delay { ms:u16 } }
#[derive(Clone, Serialize, Deserialize)]
pub struct MacroRecord { pub name:String, pub events:Vec<MacroEvent> }

#[derive(Clone,Copy,PartialEq,Debug)]
pub enum KeyCategory { Letters,Umlauts,Digits,Function,Control,Navigation,Symbols,Numpad }
impl KeyCategory {
    pub const ALL:[Self;8]=[Self::Letters,Self::Umlauts,Self::Digits,Self::Function,Self::Control,Self::Navigation,Self::Symbols,Self::Numpad];
    pub fn label(self)->&'static str {match self {
        Self::Letters=>crate::i18n::text("Buchstaben"),Self::Umlauts=>crate::i18n::text(match crate::keyboard::current(){crate::keyboard::Layout::GermanIso=>"Umlaute",crate::keyboard::Layout::SpanishIso=>"Ñ und Akzente",_=>"Layout-Zeichen"}),Self::Digits=>crate::i18n::text("Zahlen"),Self::Function=>crate::i18n::text("Funktionstasten"),
        Self::Control=>crate::i18n::text("Steuerung"),Self::Navigation=>"Navigation",Self::Symbols=>crate::i18n::text("Sonderzeichen"),Self::Numpad=>crate::i18n::text("Ziffernblock"),
    }}
    pub fn keys(self)->Vec<u8> {match self {
        Self::Letters=>{let mut keys:Vec<_>=(4..=29).collect();if crate::keyboard::current()==crate::keyboard::Layout::FrenchIso{keys.retain(|key|*key!=16);keys.push(51);keys.sort_by_key(|key|key_label(*key));}keys},
        Self::Umlauts=>match crate::keyboard::current(){crate::keyboard::Layout::FrenchIso=>vec![31,36,38,39,52],crate::keyboard::Layout::NordicIso=>vec![47,51,52],_=>vec![52,51,47,45]},Self::Digits=>(30..=39).collect(),
        Self::Function=>(58..=69).chain(104..=115).collect(),
        Self::Control=>{let mut keys=vec![224,228,225,229,226,230,227,231,40,41,42,43,44,57,101];if crate::keyboard::current()==crate::keyboard::Layout::JapaneseJis{keys.extend([136,138,139]);}keys},
        Self::Navigation=>(70..=82).collect(),Self::Symbols=>{let mut keys=vec![46,48,49,50,53,54,55,56,100];match crate::keyboard::current(){crate::keyboard::Layout::JapaneseJis=>keys.extend([135,137]),crate::keyboard::Layout::FrenchIso=>keys.push(16),_=>{}}keys},Self::Numpad=>(83..=99).collect(),
    }}
}

pub fn insert_macro_key(events:&mut Vec<MacroEvent>,hid:u8,action:u8)->Result<(),String> {
    if !KeyCategory::ALL.iter().any(|category|category.keys().contains(&hid)) {return Err(crate::i18n::text("Unbekannte Makrotaste").into());}
    let added=match action {
        0=>vec![MacroEvent::Key{hid,down:true},MacroEvent::Delay{ms:30},MacroEvent::Key{hid,down:false}],
        1=>vec![MacroEvent::Key{hid,down:true}],2=>vec![MacroEvent::Key{hid,down:false}],
        _=>return Err(crate::i18n::text("Unbekannte Makroaktion").into()),
    };
    if events.len()+added.len()>1000 {return Err(crate::i18n::text("Das Makro hat die maximale Länge erreicht").into());}
    events.extend(added);Ok(())
}

pub fn key_label(hid:u8) -> String {
    if let Some(label)=crate::keyboard::current().legend(hid){return label.into();}
    let special = match hid { 4=>"A",5=>"B",6=>"C",7=>"D",8=>"E",9=>"F",10=>"G",11=>"H",12=>"I",13=>"J",14=>"K",15=>"L",16=>"M",17=>"N",18=>"O",19=>"P",20=>"Q",21=>"R",22=>"S",23=>"T",24=>"U",25=>"V",26=>"W",27=>"X",28=>"Z",29=>"Y",40=>"Enter",41=>"Esc",42=>"Backspace",43=>"Tab",44=>crate::i18n::text("Leertaste"),45=>"ß",46=>"´",47=>"Ü",48=>"+",49=>"\\",50=>"#",51=>"Ö",52=>"Ä",53=>"^",54=>",",55=>".",56=>"-",57=>"Caps",100=>"<",224=>crate::i18n::text("Strg"),225=>"Shift",226=>"Alt",227=>"Win",228=>crate::i18n::text("Strg R"),229=>crate::i18n::text("Shift R"),230=>"Alt Gr",231=>crate::i18n::text("Win R"),240=>"Fn",_=>"" };
    if !special.is_empty() {return special.into();}
    match hid {
        30..=38=>(hid-29).to_string(),39=>"0".into(),58..=69=>format!("F{}",hid-57),104..=115=>format!("F{}",hid-91),
        70=>crate::i18n::text("Druck").into(),71=>crate::i18n::text("Rollen").into(),72=>"Pause".into(),73=>crate::i18n::text("Einfügen").into(),74=>crate::i18n::text("Pos1").into(),75=>crate::i18n::text("Bild ↑").into(),
        76=>crate::i18n::text("Entf").into(),77=>crate::i18n::text("Ende").into(),78=>crate::i18n::text("Bild ↓").into(),79=>crate::i18n::text("Pfeil →").into(),80=>crate::i18n::text("Pfeil ←").into(),81=>crate::i18n::text("Pfeil ↓").into(),82=>crate::i18n::text("Pfeil ↑").into(),
        83=>"Num Lock".into(),84=>"Num /".into(),85=>"Num *".into(),86=>"Num -".into(),87=>"Num +".into(),88=>"Num Enter".into(),
        89..=97=>format!("Num {}",hid-88),98=>"Num 0".into(),99=>"Num ,".into(),101=>crate::i18n::text("Menü").into(),_=>crate::i18n::format_text("Taste {hid}", &[format!("{}", hid)]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn json_profiles_require_valid_length_schema_and_crc_but_preserve_vendor_data(){
        let mut profile=Profile{bytes:vec![0;PROFILE_LEN]};profile.bytes[0]=3;profile.bytes[780]=6;
        profile.bytes[MACRO_END..CRC_OFFSET].fill(0xa5);profile.seal();
        let bytes=serde_json::to_vec(&profile).unwrap();
        assert!(serde_json::from_slice::<Profile>(&bytes).unwrap()==profile);
        profile.bytes[HIGH]^=1;
        assert!(serde_json::from_slice::<Profile>(&serde_json::to_vec(&profile).unwrap()).is_err());
        for len in [0,1,100,PROFILE_LEN-1,PROFILE_LEN+1]{
            let invalid=serde_json::json!({"bytes":vec![0u8;len]});
            assert!(serde_json::from_value::<Profile>(invalid).is_err());
        }
        profile.bytes[0]=4;profile.seal();
        assert!(serde_json::from_slice::<Profile>(&serde_json::to_vec(&profile).unwrap()).is_err());
    }
    #[test]
    fn manual_function_umlaut_and_control_macro_roundtrip() {
        let keys=KeyCategory::Function.keys();
        assert_eq!(keys.len(),24);assert_eq!(keys[12],104);assert_eq!(keys[23],115);
        assert_eq!(key_label(104),"F13");assert_eq!(key_label(115),"F24");
        assert!(KeyCategory::Umlauts.keys().contains(&52));
        let mut events=Vec::new();
        insert_macro_key(&mut events,224,1).unwrap(); // Ctrl down
        insert_macro_key(&mut events,6,0).unwrap(); // C tap
        insert_macro_key(&mut events,224,2).unwrap(); // Ctrl up
        insert_macro_key(&mut events,115,0).unwrap(); // F24 tap
        insert_macro_key(&mut events,52,0).unwrap(); // German Ä key
        let mut bytes=vec![0;PROFILE_LEN];bytes[0]=3;bytes[780]=6;bytes[MACRO_END..CRC_OFFSET].fill(0x5a);
        let mut profile=Profile{bytes};profile.seal();
        profile.assign_first_macro(&[5],&events).unwrap();
        let read=Profile::parse(profile.bytes).unwrap();
        assert_eq!(read.macros()[0].events,events);
        assert_eq!(read.value(5).unwrap().binding,[0x71,0,0,0,0]);
        assert!(read.bytes[MACRO_END..CRC_OFFSET].iter().all(|b|*b==0x5a));
    }
    #[test]
    fn unbalanced_manual_modifier_is_rejected_without_modifying_profile() {
        let mut events=Vec::new();insert_macro_key(&mut events,224,1).unwrap();
        let mut profile=Profile{bytes:vec![0;PROFILE_LEN]};let original=profile.bytes.clone();
        assert!(profile.assign_first_macro(&[5],&events).is_err());assert_eq!(profile.bytes,original);
        assert!(insert_macro_key(&mut events,0xff,0).is_err());
    }
}
