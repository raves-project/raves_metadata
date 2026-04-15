use crate::providers::png::{
    chunks::{Chunk, ChunkContext, PngChunkHeader},
    error::{PngConstructionError, PngWriteError},
};

/// The iCCP chunk provides an advanced color profile.
#[derive(Clone, Debug, PartialEq, PartialOrd, Hash)]
pub struct Iccp {
    header: PngChunkHeader,
    profile_name: String, // converting from the NUL-terminated grossness
    compression_method: CompressionMethod,
    compressed_profile: Vec<u8>, // not decompressing that. have fun nerds
}

impl Chunk for Iccp {
    const TYPE: [u8; 4] = *b"iCCP";

    fn read(
        blob: &mut &[u8],
        header: PngChunkHeader,
        context: ChunkContext,
    ) -> Result<Self, PngConstructionError> {
        // check that there's no context
        debug_assert_eq!(context, ChunkContext::Other);

        let mut profile_name_bytes: Vec<u8> = Vec::with_capacity(79);
        let mut found_null_terminator: bool = false;

        for _ in 0..79 {
            let byte: u8 = Self::read_u8(blob, "profile name (one byte)")?;
            if byte == 0x00 {
                found_null_terminator = true;
                break;
            }

            profile_name_bytes.push(byte);
        }

        if !found_null_terminator {
            let null_separator: u8 = Self::read_u8(blob, "null separator")?;
            if null_separator != 0x00 {
                return Err(PngConstructionError::IccpInvalidProfileName {
                    reason: "profile name must be terminated by a NUL separator after 1-79 bytes",
                });
            }
        }

        // make sure the profile name is up to spec
        validate_profile_name_bytes(&profile_name_bytes).map_err(|err| match err {
            ProfileNameError::NotLatin1 { c } => {
                PngConstructionError::IccpProfileNameNotLatin1 { c }
            }
            ProfileNameError::InvalidStructure { reason } => {
                PngConstructionError::IccpInvalidProfileName { reason }
            }
        })?;

        // write the profile name into a string
        let profile_name: String = profile_name_bytes.into_iter().map(char::from).collect();

        // grab compression method
        let compression_method_raw: u8 = Self::read_u8(blob, "compression method")?;
        let compression_method: CompressionMethod = if compression_method_raw != 0 {
            return Err(PngConstructionError::IccpUnknownCompressionValue {
                value: compression_method_raw,
            });
        } else {
            CompressionMethod::ZlibDeflate
        };

        // read the rest of the compressed profile
        let compressed_profile: Vec<u8> = blob.to_vec();
        *blob = &[];

        Ok(Self {
            header,
            profile_name,
            compression_method,
            compressed_profile,
        })
    }

    fn write<W: std::io::Write>(&self, buf: &mut W) -> Result<(), PngWriteError> {
        // write the header
        self.header.write(buf)?;

        let profile_name_bytes: Vec<u8> = self
            .profile_name
            .chars()
            .map(|c: char| {
                let code_point: u32 = c.into();
                u8::try_from(code_point).map_err(|_| PngWriteError::IccpInvalidProfileName {
                    reason: "profile name must contain only Latin-1 code points",
                })
            })
            .collect::<Result<Vec<u8>, PngWriteError>>()?;

        validate_profile_name_bytes(&profile_name_bytes).map_err(|err| match err {
            ProfileNameError::NotLatin1 { c } => PngWriteError::IccpProfileNameNotLatin1 { c },
            ProfileNameError::InvalidStructure { reason } => {
                PngWriteError::IccpInvalidProfileName { reason }
            }
        })?;

        // profile name
        Self::write_byte_slice(&profile_name_bytes, buf, "profile name")?;

        // NUL terminator on profile name
        Self::write_u8(0_u8, buf, "null separator")?;

        // compression method (will always be 0)
        Self::write_u8(self.compression_method as u8, buf, "compression method")?;

        // compressed profile
        Self::write_byte_slice(&self.compressed_profile, buf, "compressed profile")?;

        Ok(())
    }
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Hash)]
pub enum CompressionMethod {
    ZlibDeflate = 0,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ProfileNameError {
    NotLatin1 { c: u8 },
    InvalidStructure { reason: &'static str },
}

fn validate_profile_name_bytes(bytes: &[u8]) -> Result<(), ProfileNameError> {
    if bytes.is_empty() {
        return Err(ProfileNameError::InvalidStructure {
            reason: "profile name must contain at least one byte",
        });
    }

    if bytes.len() > 79 {
        return Err(ProfileNameError::InvalidStructure {
            reason: "profile name must be 79 bytes or fewer",
        });
    }

    let mut prev_was_space: bool = false;
    for (idx, &byte) in bytes.iter().enumerate() {
        if !profile_name_byte_is_allowed(byte) {
            return Err(ProfileNameError::NotLatin1 { c: byte });
        }

        let is_space: bool = byte == b' ';
        if (idx == 0 || idx + 1 == bytes.len()) && is_space {
            return Err(ProfileNameError::InvalidStructure {
                reason: "profile name may not have leading or trailing spaces",
            });
        }

        if prev_was_space && is_space {
            return Err(ProfileNameError::InvalidStructure {
                reason: "profile name may not contain consecutive spaces",
            });
        }

        prev_was_space = is_space;
    }

    Ok(())
}

/// Checks if a byte in the profile name is allowed.
const fn profile_name_byte_is_allowed(c: u8) -> bool {
    (c >= 0x20 && c <= 0x7E) || (c >= 0xA1)
}
