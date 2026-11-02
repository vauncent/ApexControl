use serde::{Deserialize,Serialize};
use std::{collections::HashMap,sync::{OnceLock,atomic::{AtomicU8,Ordering}}};

#[derive(Clone,Copy,Debug,Default,PartialEq,Serialize,Deserialize)]
pub enum Language {
    #[default]
    #[serde(rename="de")]
    German,
    #[serde(rename="en")]
    English,
}
impl Language {pub fn label(self)->&'static str {match self {Self::German=>"Deutsch",Self::English=>"English"}}}
static LANGUAGE:AtomicU8=AtomicU8::new(0);
#[cfg(test)]
thread_local! {static TEST_LANGUAGE:std::cell::Cell<Option<Language>>=const{std::cell::Cell::new(None)};}
pub fn language()->Language {
    #[cfg(test)]
    if let Some(language)=TEST_LANGUAGE.with(|value|value.get()){return language;}
    if LANGUAGE.load(Ordering::Relaxed)==1 {Language::English}else{Language::German}
}
pub fn set_language(language:Language){
    #[cfg(test)]
    if TEST_LANGUAGE.with(|value|value.get().is_some()){TEST_LANGUAGE.with(|value|value.set(Some(language)));return;}
    LANGUAGE.store(if language==Language::English{1}else{0},Ordering::Relaxed);
}
fn translations()->&'static HashMap<String,String> {
    static ENGLISH:OnceLock<HashMap<String,String>>=OnceLock::new();
    ENGLISH.get_or_init(||serde_json::from_str(include_str!("../locales/en.json")).expect("embedded English translations"))
}
pub fn text(source:&str)->&str {
    if language()==Language::English {translations().get(source).map(String::as_str).unwrap_or(source)}else{source}
}
// Insert already-formatted values after translation. User names and file paths
// are never translated or parsed again, even if they contain braces.
pub fn format_text(source:&str,arguments:&[String])->String {
    let mut rest=text(source);let mut result=String::new();let mut index=0;
    while let Some(start)=rest.find('{') {
        let Some(end)=rest[start..].find('}').map(|end|start+end) else{break};
        result.push_str(&rest[..start]);
        if let Some(value)=arguments.get(index){result.push_str(value);}else{result.push_str(&rest[start..=end]);}
        index+=1;rest=&rest[end+1..];
    }
    result.push_str(rest);debug_assert_eq!(index,arguments.len(),"translation argument mismatch: {source}");result
}
#[cfg(test)]
pub fn with_language<R>(language:Language,body:impl FnOnce()->R)->R {
    struct Reset(Option<Language>);
    impl Drop for Reset{fn drop(&mut self){TEST_LANGUAGE.with(|value|value.set(self.0));}}
    let _reset=Reset(TEST_LANGUAGE.with(|value|value.replace(Some(language))));body()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fields(mut value:&str)->Vec<&str> {
        let mut result=Vec::new();
        while let Some(start)=value.find('{') {let end=value[start..].find('}').unwrap()+start;result.push(&value[start..=end]);value=&value[end+1..];}
        result
    }
    #[test]
    fn every_translation_preserves_the_message_values() {
        assert!(translations().len()>150);
        for (source,english) in translations(){assert_eq!(fields(source),fields(english),"message placeholders: {source}");assert!(!english.is_empty());}
    }
    #[test]
    fn english_messages_preserve_user_text_and_german_still_works() {
        with_language(Language::English,||{
            assert_eq!(text("Tastenbelegung"),"Key bindings");
            assert_eq!(format_text("Auf PC · {}",&["Mein {Profil} Ä".into()]),"On this PC · Mein {Profil} Ä");
            assert_eq!(format_text("Tastatur meldet Fehler {} bei {opcode:02X}",&["3".into(),"B2".into()]),"Keyboard returned error 3 for B2");
            assert_eq!(text("ApexControl"),"ApexControl");
        });
        assert_eq!(text("Tastenbelegung"),"Tastenbelegung");
    }
}
