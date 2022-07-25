pub const DIRECT_HEADER_LEN: usize = 29;
pub const SIGNAL_HEADER_LEN: usize = 73;

pub fn frame_data_mtu(mtu: usize, signal: bool) -> usize {
    if signal {
        mtu - SIGNAL_HEADER_LEN
    } else {
        mtu - DIRECT_HEADER_LEN
    }
}
