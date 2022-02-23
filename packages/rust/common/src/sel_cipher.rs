pub fn decrypt(data: &[u8], _key: &[u8], _iv: u64) -> Result<Vec<u8>, ()> {
    Ok(data.to_vec())
}
pub fn encrypt(data: &[u8], _key: &[u8], _iv: u64) -> Result<Vec<u8>, ()> {
    Ok(data.to_vec())
}
