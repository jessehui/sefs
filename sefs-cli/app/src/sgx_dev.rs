use rcore_fs::dev::{DevError, DevResult, EINVAL};
use rcore_fs_sefs::dev::{SefsMac, sgx_aes_gcm_128bit_tag_t};
use rcore_fs_sefs::dev::{File, Storage};
use sgx_types::types::{Key128bit, EnclaveId, size_t, uint8_t};
use std::fs::{read_dir, remove_file};
use std::io;
use std::mem;
use std::path::*;
use sgx_tprotected_fs::{SgxFile as TfsFile, OpenOptions, EncryptMode as TfsEncryptMode};
use log::*;
use std::os::unix::fs::{FileExt};
use std::io::Write;

type sgx_status_t = sgx_types::error::SgxStatus;

pub struct SgxStorage {
    path: PathBuf,
    mode: EncryptMode,
}

pub enum EncryptMode {
    IntegrityOnly,
    EncryptWithIntegrity(Key128bit),
    Encrypt(Key128bit),
    EncryptAutoKey,
}

impl EncryptMode {
    pub fn from_parameters(
        protect_integrity: bool,
        key: &Option<String>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        match (protect_integrity, key) {
            (true, None) => Ok(EncryptMode::IntegrityOnly),
            (true, Some(key_str)) => {
                let key = Self::parse_key(&key_str)?;
                Ok(EncryptMode::EncryptWithIntegrity(key))
            }
            (false, None) => Ok(EncryptMode::EncryptAutoKey),
            (false, Some(key_str)) => {
                let key = Self::parse_key(&key_str)?;
                Ok(EncryptMode::Encrypt(key))
            }
        }
    }

    fn parse_key(key_str: &str) -> Result<Key128bit, Box<dyn std::error::Error>> {
        let bytes_str_vec = {
            let bytes_str_vec: Vec<&str> = key_str.split("-").collect();
            if bytes_str_vec.len() != std::mem::size_of::<Key128bit>() {
                return Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "The length or format of Key string is invalid",
                )));
            }
            bytes_str_vec
        };

        let mut key: Key128bit = Default::default();
        for (byte_i, byte_str) in bytes_str_vec.iter().enumerate() {
            key[byte_i] = u8::from_str_radix(byte_str, 16)?;
        }
        Ok(key)
    }
}

impl SgxStorage {
    pub fn new(path: impl AsRef<Path>, mode: EncryptMode) -> Self {
        SgxStorage {
            path: path.as_ref().to_path_buf(),
            mode,
        }
    }
}

impl Storage for SgxStorage {
    fn open(&self, file_id: &str) -> DevResult<Box<dyn File>> {
        let mut path = self.path.clone();
        path.push(file_id);
        let file = file_open(path.to_str().unwrap(), false, &self.mode)?;
        Ok(Box::new(SgxFile { file }))
    }

    fn create(&self, file_id: &str) -> DevResult<Box<dyn File>> {
        let mut path = self.path.clone();
        path.push(file_id);
        let file = file_open(path.to_str().unwrap(), true, &self.mode)?;
        Ok(Box::new(SgxFile { file }))
    }

    fn remove(&self, file_id: &str) -> DevResult<()> {
        let mut path = self.path.to_path_buf();
        path.push(file_id);
        remove_file(path)?;
        Ok(())
    }

    fn protect_integrity(&self) -> bool {
        match self.mode {
            EncryptMode::IntegrityOnly => true,
            EncryptMode::EncryptWithIntegrity(_) => true,
            _ => false,
        }
    }

    fn clear(&self) -> DevResult<()> {
        for child in read_dir(&self.path)? {
            let child = child?;
            remove_file(&child.path())?;
        }
        Ok(())
    }
}

pub struct SgxFile {
    file: TfsFile,
}

impl File for SgxFile {
    fn read_at(&self, buf: &mut [u8], offset: usize) -> DevResult<usize> {
        let len = file_read_at(&self.file, offset, buf);
        Ok(len)
    }

    fn write_at(&self, buf: &[u8], offset: usize) -> DevResult<usize> {
        let len = file_write_at(&self.file, offset, buf);
        if len != buf.len() {
            println!(
                "write_at return len: {} not equal to buf_len: {}",
                len,
                buf.len()
            );
            return Err(DevError(EINVAL));
        }
        Ok(len)
    }

    fn set_len(&self, _len: usize) -> DevResult<()> {
        // NOTE: do nothing ?
        Ok(())
    }

    fn flush(&self) -> DevResult<()> {
        match file_flush(&self.file) {
            0 => Ok(()),
            e => {
                println!("failed to flush");
                return Err(DevError(e));
            }
        }
    }

    fn get_file_mac(&self) -> DevResult<SefsMac> {
        let mut mac: sgx_aes_gcm_128bit_tag_t = [0u8; 16];

        file_get_mac(&self.file, &mut mac);
        println!("mac = {:?}", mac);
        let sefs_mac = SefsMac(mac);
        Ok(sefs_mac)
    }
}

// impl Drop for SgxFile {
//     fn drop(&mut self) {
//         let _ = file_close(&self.file);
//     }
// }

fn file_get_mac(file: &TfsFile, mac: *mut sgx_aes_gcm_128bit_tag_t) -> usize {
    let mut ret_val = 0;
    unsafe {
        let len = mem::size_of::<sgx_aes_gcm_128bit_tag_t>();
        // let ret = ecall_file_get_mac(EID, &mut ret_val, fd, mac as *mut u8, len);
        // assert_eq!(ret, sgx_status_t::Success);
        // let mut result_mac = file.get_mac().unwrap();
        // mem::swap(&mut result_mac, &mut *mac);

        let mut result_mac = file.get_mac();
        if let Ok(result) = &mut result_mac {
            mem::swap(result, &mut *mac);
        }
    } 
    ret_val as usize
}

fn file_open(path: &str, create: bool, mode: &EncryptMode) -> DevResult<TfsFile> { 
    let cpath = format!("{}\0", path);
    let (protect_integrity, key_ptr) = match mode {
        EncryptMode::IntegrityOnly => (true, std::ptr::null()),
        EncryptMode::EncryptWithIntegrity(key) => (true, key as *const Key128bit),
        EncryptMode::Encrypt(key) => (false, key as *const Key128bit),
        EncryptMode::EncryptAutoKey => (false, std::ptr::null()), 
    };
    let encrypt_mode = match mode {
        EncryptMode::IntegrityOnly => TfsEncryptMode::integrity_only(),
        _ => todo!(),
    };
    let mut ret_val = 0;
    let mut error = 0;
    let path = Path::new(path);
    // let opts = OpenOptions::new();
    let file = if create {
        TfsFile::create_integrity_only(path).unwrap()
    } else {
        TfsFile::open_integrity_only(path).unwrap()
    };
    Ok(file)
}

fn file_flush(file: &TfsFile) -> i32 {
    let mut ret_val = 0;
    file.flush(); 
    ret_val
}

fn file_read_at(file: &TfsFile, offset: usize, buf: &mut [u8]) -> usize {
    let ret_val = file.read_at(buf, offset as u64).unwrap();
    ret_val
}

fn file_write_at(file: &TfsFile, offset: usize, buf: &[u8]) -> usize {
    let ret_val = file.write_at(buf, offset as u64).unwrap();
    ret_val
}
