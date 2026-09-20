#[repr(C, align(64))]
pub struct AlignedElf<const N: usize>(pub [u8; N]);
