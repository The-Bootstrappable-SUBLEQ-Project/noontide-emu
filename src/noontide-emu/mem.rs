pub fn read(mem: &[u8], offset: usize) -> i64 {
    i64::from_be_bytes(mem[offset..offset + 8].try_into().unwrap())
}

pub fn write(mem: &mut [u8], offset: usize, data: &[u8; 8]) {
    mem[offset..offset + 8].clone_from_slice(data);
}
