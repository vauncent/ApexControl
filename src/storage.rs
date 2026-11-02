//! Bounded local input and same-directory, exclusive temporary files.
use std::{fs::{self,File,OpenOptions},io::{self,Read,Write},path::Path,sync::atomic::{AtomicU64,Ordering}};

pub const SETTINGS_LIMIT:usize=64*1024;
pub const PROFILE_LIMIT:usize=1024*1024;
pub const LIBRARY_LIMIT:usize=32*1024*1024;

fn invalid(message:&str)->io::Error{io::Error::new(io::ErrorKind::InvalidData,crate::i18n::text(message))}

pub fn read(path:&Path,limit:usize)->io::Result<Vec<u8>> {
    // Reject links and special files before opening, then check the opened file
    // again. The byte limit also applies if a regular file grows during reading.
    let metadata=fs::symlink_metadata(path)?;
    if !metadata.is_file() || metadata.file_type().is_symlink(){return Err(invalid("Keine reguläre Datendatei"));}
    let file=File::open(path)?;let metadata=file.metadata()?;
    if !metadata.is_file(){return Err(invalid("Keine reguläre Datendatei"));}
    if metadata.len()>limit as u64{return Err(invalid("Die Datendatei ist zu groß"));}
    read_bounded(file,limit)
}
fn read_bounded(reader:impl Read,limit:usize)->io::Result<Vec<u8>> {
    let mut bytes=Vec::new();reader.take(limit as u64+1).read_to_end(&mut bytes)?;
    if bytes.len()>limit{return Err(invalid("Die Datendatei ist zu groß"));}Ok(bytes)
}

pub fn write(path:&Path,bytes:&[u8],limit:usize)->io::Result<()> {
    if bytes.len()>limit{return Err(invalid("Die Datendatei ist zu groß"));}
    let folder=path.parent().ok_or_else(||invalid("Ungültiger Speicherpfad"))?;
    fs::create_dir_all(folder)?;
    // Never truncate a predictable .tmp path: another process could have put a
    // link there. create_new fails on existing files, including dangling links.
    static NEXT:AtomicU64=AtomicU64::new(0);
    for _ in 0..32 {
        let temp=folder.join(format!(".apexcontrol-{}-{}.tmp",std::process::id(),NEXT.fetch_add(1,Ordering::Relaxed)));
        let mut options=OpenOptions::new();options.write(true).create_new(true);
        #[cfg(unix)] {use std::os::unix::fs::OpenOptionsExt;options.mode(0o600);}
        let mut file=match options.open(&temp){Ok(file)=>file,Err(e) if e.kind()==io::ErrorKind::AlreadyExists=>continue,Err(e)=>return Err(e)};
        let result=(||{file.write_all(bytes)?;file.sync_all()?;drop(file);fs::rename(&temp,path)})();
        if result.is_err(){let _=fs::remove_file(&temp);}return result;
    }
    Err(io::Error::new(io::ErrorKind::AlreadyExists,"Cannot create an exclusive temporary file"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn growing_or_oversized_input_is_bounded_even_without_metadata(){
        assert!(read_bounded(io::repeat(0),1024).is_err());
        assert_eq!(read_bounded(&b"abcd"[..],4).unwrap(),b"abcd");
    }
    #[test]
    fn atomic_save_preserves_original_on_rejected_data_and_ignores_old_temp(){
        let folder=std::env::temp_dir().join(format!("apexcontrol-storage-{}",std::process::id()));
        fs::create_dir_all(&folder).unwrap();let path=folder.join("settings.json");
        fs::write(folder.join("settings.tmp"),b"do not touch").unwrap();
        write(&path,b"original",32).unwrap();
        assert!(write(&path,b"too long",2).is_err());assert_eq!(read(&path,32).unwrap(),b"original");
        write(&path,b"replacement",32).unwrap();assert_eq!(read(&path,32).unwrap(),b"replacement");
        assert_eq!(fs::read(folder.join("settings.tmp")).unwrap(),b"do not touch");
        assert!(read(&folder,32).is_err());fs::remove_dir_all(folder).unwrap();
    }
}
