use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Key, Nonce,
};
use anyhow::{anyhow, Result};
#[allow(dead_code)]
pub const KEY_SIZE: usize   = 32;
pub const NONCE_SIZE: usize = 12;

#[derive(Clone, Debug)]
pub struct RoomKey {
    pub bytes: Vec<u8>,
}

impl RoomKey {
    pub fn generate() -> Self {
        let key = Aes256Gcm::generate_key(OsRng);
        Self { bytes: key.to_vec() }
    }

    pub fn to_hex(&self) -> String {
        self.bytes.iter().map(|b| format!("{:02x}", b)).collect()
    }

    #[allow(dead_code)]
    pub fn from_hex(hex: &str) -> Result<Self> {
        let bytes = (0..hex.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&hex[i..i + 2], 16))
            .collect::<Result<Vec<u8>, _>>()
            .map_err(|e| anyhow!("Invalid hex: {}", e))?;
        Ok(Self { bytes })
    }
}

pub fn encrypt(plaintext: &str, room_key: &RoomKey) -> Result<String> {
    let key    = Key::<Aes256Gcm>::from_slice(&room_key.bytes);
    let cipher = Aes256Gcm::new(key);
    let nonce  = Aes256Gcm::generate_nonce(&mut OsRng);

    let ciphertext = cipher
        .encrypt(&nonce, plaintext.as_bytes())
        .map_err(|e| anyhow!("Encryption failed: {}", e))?;

    let mut combined = Vec::with_capacity(NONCE_SIZE + ciphertext.len());
    combined.extend_from_slice(&nonce);
    combined.extend_from_slice(&ciphertext);

    Ok(base64_encode(&combined))
}
#[allow(dead_code)]
pub fn decrypt(encrypted: &str, room_key: &RoomKey) -> Result<String> {
    let combined = base64_decode(encrypted)?;

    if combined.len() < NONCE_SIZE {
        return Err(anyhow!("Data too short"));
    }

    let (nonce_bytes, ciphertext) = combined.split_at(NONCE_SIZE);
    let key    = Key::<Aes256Gcm>::from_slice(&room_key.bytes);
    let cipher = Aes256Gcm::new(key);
    let nonce  = Nonce::from_slice(nonce_bytes);

    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| anyhow!("Decryption failed: {}", e))?;

    String::from_utf8(plaintext)
        .map_err(|e| anyhow!("Invalid UTF-8: {}", e))
}

pub fn base64_encode(data: &[u8]) -> String {
    const CHARS: &[u8] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::new();
    let mut i = 0;
    while i < data.len() {
        let b0 = data[i] as usize;
        let b1 = if i + 1 < data.len() { data[i + 1] as usize } else { 0 };
        let b2 = if i + 2 < data.len() { data[i + 2] as usize } else { 0 };
        result.push(CHARS[b0 >> 2] as char);
        result.push(CHARS[((b0 & 3) << 4) | (b1 >> 4)] as char);
        result.push(if i + 1 < data.len() {
            CHARS[((b1 & 15) << 2) | (b2 >> 6)] as char
        } else { '=' });
        result.push(if i + 2 < data.len() {
            CHARS[b2 & 63] as char
        } else { '=' });
        i += 3;
    }
    result
}
#[allow(dead_code)]
pub fn base64_decode(data: &str) -> Result<Vec<u8>> {
    const DECODE: [i8; 128] = {
        let mut t = [-1i8; 128];
        let mut i = 0u8;
        while i < 26 { t[(b'A' + i) as usize] = i as i8; i += 1; }
        let mut i = 0u8;
        while i < 26 { t[(b'a' + i) as usize] = (26 + i) as i8; i += 1; }
        let mut i = 0u8;
        while i < 10 { t[(b'0' + i) as usize] = (52 + i) as i8; i += 1; }
        t[b'+' as usize] = 62;
        t[b'/' as usize] = 63;
        t
    };
    let data  = data.trim_end_matches('=');
    let bytes: Vec<u8> = data.bytes().collect();
    let mut result = Vec::new();
    let mut i = 0;
    while i + 1 < bytes.len() {
        let b0 = DECODE[bytes[i] as usize];
        let b1 = DECODE[bytes[i + 1] as usize];
        if b0 < 0 || b1 < 0 { return Err(anyhow!("Invalid base64")); }
        result.push(((b0 << 2) | (b1 >> 4)) as u8);
        if i + 2 < bytes.len() {
            let b2 = DECODE[bytes[i + 2] as usize];
            if b2 >= 0 { result.push(((b1 << 4) | (b2 >> 2)) as u8); }
        }
        if i + 3 < bytes.len() {
            let b3 = DECODE[bytes[i + 3] as usize];
            if b3 >= 0 {
                result.push(((DECODE[bytes[i + 2] as usize] << 6) | b3) as u8);
            }
        }
        i += 4;
    }
    Ok(result)
}