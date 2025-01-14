// //! Big number types

// #![allow(clippy::assign_op_pattern)]
// #![allow(clippy::ptr_offset_with_cast)]
// #![allow(clippy::manual_range_contains)]

// use crate::error::ErrorCode::BnConversionError;
// // use std::borrow::BorrowMut;
// // use std::convert::TryInto;
// // use std::mem::size_of;
// // use uint::construct_uint;

// use crate::error::NormalResult;

// // construct_uint! {
// //     /// 256-bit unsigned integer.
// //     pub struct U256(4);
// // }

// /// A 256-bit unsigned integer represented as an array of 4 `u64` values.
// #[derive(Clone, Debug, PartialEq, Eq)]
// pub struct U256([u64; 4]);

// impl U256 {
//    /// Convert a U256 to a u64 if it fits.
//    pub fn to_u64(&self) -> Option<u64> {
//     if self.0[1] == 0 && self.0[2] == 0 && self.0[3] == 0 {
//         Some(self.0[0])
//     } else {
//         None
//     }
// }

// /// Convert a U256 to a u128 if it fits.
// pub fn to_u128(&self) -> Option<u128> {
//     if self.0[2] == 0 && self.0[3] == 0 {
//         Some((self.0[1] as u128) << 64 | (self.0[0] as u128))
//     } else {
//         None
//     }
// }

// /// Create a U256 from little-endian bytes.
// pub fn from_le_bytes(bytes: BytesN<32>) -> Self {
//     let mut words = [0u64; 4];
//     for (i, chunk) in bytes.as_slice().chunks(8).enumerate() {
//         words[i] = u64::from_le_bytes(chunk.try_into().unwrap_or_default());
//     }
//     U256(words)
// }

// /// Convert a U256 to little-endian bytes.
// pub fn to_le_bytes(&self, env: &Env) -> BytesN<32> {
//     let mut bytes = [0u8; 32];
//     for (i, word) in self.0.iter().enumerate() {
//         bytes[i * 8..(i + 1) * 8].copy_from_slice(&word.to_le_bytes());
//     }
//     BytesN::from_array(env, &bytes)
// }
// }

// // construct_uint! {
// //     /// 192-bit unsigned integer.
// //     pub struct U192(3);
// // }
// /// A 256-bit unsigned integer represented as an array of 4 `u64` values.
// #[derive(Clone, Debug, PartialEq, Eq)]
// pub struct U192([u64; 3]);

// impl U192 {
//     /// Convert u192 to u64
//     pub fn to_u64(self) -> Option<u64> {
//         self.try_to_u64().map_or_else(|_| None, Some)
//     }

//     /// Convert u192 to u64
//     pub fn try_to_u64(self) -> NormalResult<u64> {
//         self.try_into().map_err(|_| BnConversionError)
//     }

//     /// Convert u192 to u128
//     pub fn to_u128(self) -> Option<u128> {
//         self.try_to_u128().map_or_else(|_| None, Some)
//     }

//     /// Convert u192 to u128
//     pub fn try_to_u128(self) -> NormalResult<u128> {
//         self.try_into().map_err(|_| BnConversionError)
//     }

//     /// Convert from little endian bytes
//     pub fn from_le_bytes(bytes: [u8; 24]) -> Self {
//         U192::from_little_endian(&bytes)
//     }

//     /// Convert to little endian bytes
//     pub fn to_le_bytes(self) -> [u8; 24] {
//         let mut buf: Vec<u8> = Vec::with_capacity(size_of::<Self>());
//         self.to_little_endian(buf.borrow_mut());

//         let mut bytes: [u8; 24] = [0u8; 24];
//         bytes.copy_from_slice(buf.as_slice());
//         bytes
//     }
// }
