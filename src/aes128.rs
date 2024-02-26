use std::convert::AsMut;
const MATRIX_SIZE: usize = 4;
const ROUND_COUNT: usize = 11;
//The AES sbox
static AES_SBOX: [[u8; MATRIX_SIZE * MATRIX_SIZE]; MATRIX_SIZE * MATRIX_SIZE] = [
    [
        0x63, 0x7c, 0x77, 0x7b, 0xf2, 0x6b, 0x6f, 0xc5, 0x30, 0x01, 0x67, 0x2b, 0xfe, 0xd7, 0xab,
        0x76,
    ],
    [
        0xca, 0x82, 0xc9, 0x7d, 0xfa, 0x59, 0x47, 0xf0, 0xad, 0xd4, 0xa2, 0xaf, 0x9c, 0xa4, 0x72,
        0xc0,
    ],
    [
        0xb7, 0xfd, 0x93, 0x26, 0x36, 0x3f, 0xf7, 0xcc, 0x34, 0xa5, 0xe5, 0xf1, 0x71, 0xd8, 0x31,
        0x15,
    ],
    [
        0x04, 0xc7, 0x23, 0xc3, 0x18, 0x96, 0x05, 0x9a, 0x07, 0x12, 0x80, 0xe2, 0xeb, 0x27, 0xb2,
        0x75,
    ],
    [
        0x09, 0x83, 0x2c, 0x1a, 0x1b, 0x6e, 0x5a, 0xa0, 0x52, 0x3b, 0xd6, 0xb3, 0x29, 0xe3, 0x2f,
        0x84,
    ],
    [
        0x53, 0xd1, 0x00, 0xed, 0x20, 0xfc, 0xb1, 0x5b, 0x6a, 0xcb, 0xbe, 0x39, 0x4a, 0x4c, 0x58,
        0xcf,
    ],
    [
        0xd0, 0xef, 0xaa, 0xfb, 0x43, 0x4d, 0x33, 0x85, 0x45, 0xf9, 0x02, 0x7f, 0x50, 0x3c, 0x9f,
        0xa8,
    ],
    [
        0x51, 0xa3, 0x40, 0x8f, 0x92, 0x9d, 0x38, 0xf5, 0xbc, 0xb6, 0xda, 0x21, 0x10, 0xff, 0xf3,
        0xd2,
    ],
    [
        0xcd, 0x0c, 0x13, 0xec, 0x5f, 0x97, 0x44, 0x17, 0xc4, 0xa7, 0x7e, 0x3d, 0x64, 0x5d, 0x19,
        0x73,
    ],
    [
        0x60, 0x81, 0x4f, 0xdc, 0x22, 0x2a, 0x90, 0x88, 0x46, 0xee, 0xb8, 0x14, 0xde, 0x5e, 0x0b,
        0xdb,
    ],
    [
        0xe0, 0x32, 0x3a, 0x0a, 0x49, 0x06, 0x24, 0x5c, 0xc2, 0xd3, 0xac, 0x62, 0x91, 0x95, 0xe4,
        0x79,
    ],
    [
        0xe7, 0xc8, 0x37, 0x6d, 0x8d, 0xd5, 0x4e, 0xa9, 0x6c, 0x56, 0xf4, 0xea, 0x65, 0x7a, 0xae,
        0x08,
    ],
    [
        0xba, 0x78, 0x25, 0x2e, 0x1c, 0xa6, 0xb4, 0xc6, 0xe8, 0xdd, 0x74, 0x1f, 0x4b, 0xbd, 0x8b,
        0x8a,
    ],
    [
        0x70, 0x3e, 0xb5, 0x66, 0x48, 0x03, 0xf6, 0x0e, 0x61, 0x35, 0x57, 0xb9, 0x86, 0xc1, 0x1d,
        0x9e,
    ],
    [
        0xe1, 0xf8, 0x98, 0x11, 0x69, 0xd9, 0x8e, 0x94, 0x9b, 0x1e, 0x87, 0xe9, 0xce, 0x55, 0x28,
        0xdf,
    ],
    [
        0x8c, 0xa1, 0x89, 0x0d, 0xbf, 0xe6, 0x42, 0x68, 0x41, 0x99, 0x2d, 0x0f, 0xb0, 0x54, 0xbb,
        0x16,
    ],
];
//the inverse AES sbox
static INVERSE_AES_SBOX: [[u8; MATRIX_SIZE * MATRIX_SIZE]; MATRIX_SIZE * MATRIX_SIZE] = [
    [
        0x52, 0x09, 0x6a, 0xd5, 0x30, 0x36, 0xa5, 0x38, 0xbf, 0x40, 0xa3, 0x9e, 0x81, 0xf3, 0xd7,
        0xfb,
    ],
    [
        0x7c, 0xe3, 0x39, 0x82, 0x9b, 0x2f, 0xff, 0x87, 0x34, 0x8e, 0x43, 0x44, 0xc4, 0xde, 0xe9,
        0xcb,
    ],
    [
        0x54, 0x7b, 0x94, 0x32, 0xa6, 0xc2, 0x23, 0x3d, 0xee, 0x4c, 0x95, 0x0b, 0x42, 0xfa, 0xc3,
        0x4e,
    ],
    [
        0x08, 0x2e, 0xa1, 0x66, 0x28, 0xd9, 0x24, 0xb2, 0x76, 0x5b, 0xa2, 0x49, 0x6d, 0x8b, 0xd1,
        0x25,
    ],
    [
        0x72, 0xf8, 0xf6, 0x64, 0x86, 0x68, 0x98, 0x16, 0xd4, 0xa4, 0x5c, 0xcc, 0x5d, 0x65, 0xb6,
        0x92,
    ],
    [
        0x6c, 0x70, 0x48, 0x50, 0xfd, 0xed, 0xb9, 0xda, 0x5e, 0x15, 0x46, 0x57, 0xa7, 0x8d, 0x9d,
        0x84,
    ],
    [
        0x90, 0xd8, 0xab, 0x00, 0x8c, 0xbc, 0xd3, 0x0a, 0xf7, 0xe4, 0x58, 0x05, 0xb8, 0xb3, 0x45,
        0x06,
    ],
    [
        0xd0, 0x2c, 0x1e, 0x8f, 0xca, 0x3f, 0x0f, 0x02, 0xc1, 0xaf, 0xbd, 0x03, 0x01, 0x13, 0x8a,
        0x6b,
    ],
    [
        0x3a, 0x91, 0x11, 0x41, 0x4f, 0x67, 0xdc, 0xea, 0x97, 0xf2, 0xcf, 0xce, 0xf0, 0xb4, 0xe6,
        0x73,
    ],
    [
        0x96, 0xac, 0x74, 0x22, 0xe7, 0xad, 0x35, 0x85, 0xe2, 0xf9, 0x37, 0xe8, 0x1c, 0x75, 0xdf,
        0x6e,
    ],
    [
        0x47, 0xf1, 0x1a, 0x71, 0x1d, 0x29, 0xc5, 0x89, 0x6f, 0xb7, 0x62, 0x0e, 0xaa, 0x18, 0xbe,
        0x1b,
    ],
    [
        0xfc, 0x56, 0x3e, 0x4b, 0xc6, 0xd2, 0x79, 0x20, 0x9a, 0xdb, 0xc0, 0xfe, 0x78, 0xcd, 0x5a,
        0xf4,
    ],
    [
        0x1f, 0xdd, 0xa8, 0x33, 0x88, 0x07, 0xc7, 0x31, 0xb1, 0x12, 0x10, 0x59, 0x27, 0x80, 0xec,
        0x5f,
    ],
    [
        0x60, 0x51, 0x7f, 0xa9, 0x19, 0xb5, 0x4a, 0x0d, 0x2d, 0xe5, 0x7a, 0x9f, 0x93, 0xc9, 0x9c,
        0xef,
    ],
    [
        0xa0, 0xe0, 0x3b, 0x4d, 0xae, 0x2a, 0xf5, 0xb0, 0xc8, 0xeb, 0xbb, 0x3c, 0x83, 0x53, 0x99,
        0x61,
    ],
    [
        0x17, 0x2b, 0x04, 0x7e, 0xba, 0x77, 0xd6, 0x26, 0xe1, 0x69, 0x14, 0x63, 0x55, 0x21, 0x0c,
        0x7d,
    ],
];
//AES rcon table
static RC: [u8; ROUND_COUNT] = [
    0x00, 0x01, 0x02, 0x04, 0x08, 0x10, 0x20, 0x40, 0x80, 0x1B, 0x36,
];
static MIX_CONST: [u8; MATRIX_SIZE] = [2, 1, 1, 3];
static INVERSE_MIX_CONST: [u8; MATRIX_SIZE] = [14, 9, 13, 11];

fn clone_into_array<A, T>(slice: &[T]) -> A
where
    A: Default + AsMut<[T]>,
    T: Clone,
{
    let mut a = A::default();
    <A as AsMut<[T]>>::as_mut(&mut a).clone_from_slice(slice);
    a
}

/// # Description
/// The function takes a key and generates all corresponding the sub keys
///
/// # Arguments
///
/// * `key_bytes`: the key as a byte array
///
/// returns: [[u8;MATRIX_SIZE];MATRIX_SIZE*ROUND_COUNT]
fn key_schedule_aes128(key_bytes: &[u8; 16]) -> [[u8; MATRIX_SIZE]; MATRIX_SIZE * ROUND_COUNT] {
    let mut original_key = [[0u8; MATRIX_SIZE]; MATRIX_SIZE];
    let mut expanded_key = [[0u8; MATRIX_SIZE]; 44];

    // create matrix
    for i in 0..MATRIX_SIZE * MATRIX_SIZE {
        original_key[i / MATRIX_SIZE][i % MATRIX_SIZE] = key_bytes[i];
    }

    for i in 0..MATRIX_SIZE * ROUND_COUNT {
        // 11 rounds, i in 0..4*rounds-1

        if i < MATRIX_SIZE {
            // original_key_bytes?
            expanded_key[i] = original_key[i];
        } else if i >= MATRIX_SIZE && i % MATRIX_SIZE == 0 {
            //is the first sub key of a generation round?

            let mut rcon = [0u8; MATRIX_SIZE];
            rcon[0] = RC[i / MATRIX_SIZE];
            expanded_key[i] = xor_words(
                &xor_words(
                    &expanded_key[i - MATRIX_SIZE],
                    &substitute_word(&rotate_word(&expanded_key[i - 1])),
                ),
                &rcon,
            );
        } else {
            // sub key of a generation round?
            expanded_key[i] = xor_words(&expanded_key[i - MATRIX_SIZE], &expanded_key[i - 1]);
        }
    }

    return expanded_key;
}

/// # Description
///     function takes a byte splits it in half and returns it's lookup in the sbox or the inverse sbox depending on the encryption flag
///
/// # Arguments
///
/// * `byte`: reference to byte to substitute
/// * `encryption`: look up in the sbox or the inverse sbox?
///
/// returns: u8
///

fn substitute(byte: &u8, encryption: bool) -> u8 {
    //split byte in half
    let upper_nibble: usize = ((byte >> MATRIX_SIZE) & 0xF).into();
    let lower_nibble: usize = (byte & 0xF).into();

    return match encryption {
        true => AES_SBOX[upper_nibble][lower_nibble],
        false => INVERSE_AES_SBOX[upper_nibble][lower_nibble],
    };
}

/// # Description
/// function takes a word (four bytes) and returns a clone of the word shifted once
/// # Arguments
///
/// * `word`: word to shift
///
/// returns: [u8; 4]
///
fn rotate_word(word: &[u8; MATRIX_SIZE]) -> [u8; MATRIX_SIZE] {
    let mut result = [0u8; MATRIX_SIZE];

    for i in 0..MATRIX_SIZE {
        result[i] = word[(i + 1) % MATRIX_SIZE];
    }

    return result;
}

/// # Description
///  function takes a word (four bytes) and returns a clone that every byte in it was subed
/// # Arguments
///
/// * `word`: word to be cloned and subed
///
/// returns: [u8; 4]
///
fn substitute_word(word: &[u8; MATRIX_SIZE]) -> [u8; MATRIX_SIZE] {
    let mut result = [0u8; MATRIX_SIZE];

    for i in 0..MATRIX_SIZE {
        result[i] = substitute(&word[i], true);
    }

    return result;
}

/// # Description
/// function takes two words(four bytes) and returns a word with result of xor their bytes
/// # Arguments
///
/// * `word1`: word(four bytes) to be xored with word2
/// * `word2`: word(four bytes) to be xored with word1
///
/// returns: [u8; 4]
///

fn xor_words(word1: &[u8; MATRIX_SIZE], word2: &[u8; MATRIX_SIZE]) -> [u8; MATRIX_SIZE] {
    let mut result = [0; MATRIX_SIZE];
    for i in 0..4 {
        result[i] = word1[i] ^ word2[i];
    }

    return result;
}

/// # Description
/// function adds round key to a matrix by xoring their values
/// # Arguments
///
/// * `state`: matrix(4x4) to have the key added to it
/// * `key`: key to be added to state (matrix4x4)
///
/// returns: ()
///

fn add_round_key(
    state: &mut [[u8; MATRIX_SIZE]; MATRIX_SIZE],
    key: &[[u8; MATRIX_SIZE]; MATRIX_SIZE],
) {
    for i in 0..MATRIX_SIZE {
        for j in 0..MATRIX_SIZE {
            state[i][j] = state[i][j] ^ key[j][i];
        }
    }
}

/// # Description
/// function executes the substitute bytes part of the aes round function on the state matrix(4x4)
///
/// modifies state
/// # Arguments
///
/// * `state`: 4x4 matrix to be substituted
///

fn substitute_bytes(state: &mut [[u8; MATRIX_SIZE]; MATRIX_SIZE]) {
    for i in 0..MATRIX_SIZE {
        for j in 0..MATRIX_SIZE {
            state[i][j] = substitute(&state[i][j], true);
        }
    }
}
/// # Description
/// function executes the substitute bytes part of the AES decrypt round function on the state matrix(4x4)
///
/// modifies state
/// # Arguments
///
/// * `state`: 4x4 matrix to be substituted
///
fn inverse_substitute_bytes(state: &mut [[u8; MATRIX_SIZE]; MATRIX_SIZE]) {
    for i in 0..MATRIX_SIZE {
        for j in 0..MATRIX_SIZE {
            state[i][j] = substitute(&state[i][j], false);
        }
    }
}
/// # Description
/// function executes the shift rows part of the AES encrypt round function on the state matrix(4x4)
///
/// modifies state
/// # Arguments
///
/// * `state`: 4x4 matrix to be shifted
///
fn shift_rows(state: &mut [[u8; MATRIX_SIZE]; MATRIX_SIZE]) {
    state
        .iter_mut()
        .enumerate()
        .for_each(|(i, arr)| arr.rotate_left(i));
}
/// # Description
/// function executes the shift rows part of the AES decrypt round function on the state matrix(4x4)
///
/// modifies state
/// # Arguments
///
/// * `state`: 4x4 matrix to be un shifted
///
fn inverse_shift_rows(state: &mut [[u8; MATRIX_SIZE]; MATRIX_SIZE]) {
    state
        .iter_mut()
        .enumerate()
        .for_each(|(i, arr)| arr.rotate_right(i));
}

///# Description
/// multiplies ap in bp using Galois Field arithmetic
/// code from: https://github.com/adrgs/rust-aes/blob/master/src/aes.rs
/// explanation of what Galois Field arithmetic is : https://en.wikipedia.org/wiki/Finite_field_arithmetic
/// # Arguments
///
/// * `ap`: first number in the Galois Field
/// * `bp`: second number in the  Galois Field
///
/// returns: u8
///
fn galois_multiplication(ap: u8, bp: u8) -> u8 {
    let mut p = 0u8;
    let mut high_bit = 0u8;
    let mut a = ap;
    let mut b = bp;
    for i in 0..8 {
        if b & 1 == 1 {
            p ^= a
        }
        high_bit = a & 0x80;
        a = (a << 1) & 0xFF;
        if high_bit == 0x80 {
            a ^= 0x1b;
        }
        b = (b >> 1) & 0xFF;
    }
    return p & 0xFF;
}
/// # Description
/// function executes the mix rows part of the AES encrypt round function or the decrypt round function depending on the encryption flag on the state matrix(4x4)
///
/// modifies state
/// # Arguments
///
/// * `state`: 4x4 matrix to be un shifted
/// * `encryption` : encrypt the data or decrypt it?
fn mix_columns(state: &mut [[u8; MATRIX_SIZE]; MATRIX_SIZE], encryption: bool) {
    for i in 0..MATRIX_SIZE {
        //get a row
        let mut row = [0u8; MATRIX_SIZE];
        for j in 0..MATRIX_SIZE {
            row[j] = state[j][i];
        }

        //prep row for mixing
        row.reverse();
        row.rotate_right(1);

        state.iter_mut().for_each(|arr| {
            arr[i] = row
                .iter()
                .zip(if encryption {
                    MIX_CONST
                } else {
                    INVERSE_MIX_CONST
                })
                .map(|(&cell, const_value)| galois_multiplication(cell, const_value))
                .fold(0u8, |acc, x| acc ^ x);
            row.rotate_right(1);
        })
    }
}

pub fn encrypt_aes128(key_bytes: &[u8; 16], bytes: &[u8]) -> Vec<u8> {
    if bytes.len() % 16 != 0 {
        panic!("Input is not multiple of 16 bytes!");
    }
    let expanded_key = key_schedule_aes128(key_bytes);
    let mut result = vec![0u8; bytes.len()];

    for i in 0..bytes.len() / 16 {
        let mut block = [0u8; 16];
        for j in 0..16 {
            block[j] = bytes[i * 16 + j];
        }
        block = encrypt_block_aes128(&expanded_key, &block);
        for j in 0..16 {
            result[i * 16 + j] = block[j];
        }
    }

    return result;
}

/// # Description
/// function takes expended key and encrypts a block of bytes using AES
/// # Arguments
///
/// * `expanded_key`: expended key from the with  key_schedule_aes128  that was called with the original key
/// * `bytes`: block to encrypt
///
/// returns: [u8; 16]
///

fn encrypt_block_aes128(expanded_key: &[[u8; 4]; 44], bytes: &[u8; 16]) -> [u8; 16] {
    let mut result = [0u8; 16];

    let mut state = [[0u8; 4]; 4];
    for i in 0..16 {
        state[i % 4][i / 4] = bytes[i];
    }

    add_round_key(&mut state, &clone_into_array(&expanded_key[0..4]));

    for i in 1..10 {
        substitute_bytes(&mut state);
        shift_rows(&mut state);
        mix_columns(&mut state, true);
        add_round_key(
            &mut state,
            &clone_into_array(&expanded_key[i * 4..(i + 1) * 4]),
        );
    }

    substitute_bytes(&mut state);
    shift_rows(&mut state);
    add_round_key(&mut state, &clone_into_array(&expanded_key[40..44]));

    for i in 0..4 {
        for j in 0..4 {
            result[4 * j + i] = state[i][j]
        }
    }

    return result;
}

/// # Description
/// function decrypts bytes from aes using the key and returns the new decoded bytes
/// # Arguments
///
/// * `key_bytes`: key as bytes
/// * `bytes`: encrypted aes data
///
/// returns: Vec<u8, Global>
///
/// # Examples
///
pub fn decrypt_aes128(key_bytes: &[u8; MATRIX_SIZE * MATRIX_SIZE], bytes: &[u8]) -> Vec<u8> {
    if bytes.len() % MATRIX_SIZE * MATRIX_SIZE != 0 {
        panic!("Input is not multiple of 16 bytes!");
    }
    let expanded_key = key_schedule_aes128(key_bytes);
    let mut result = vec![0u8; bytes.len()];

    for i in 0..bytes.len() / (MATRIX_SIZE * MATRIX_SIZE) {
        let mut block = [0u8; MATRIX_SIZE * MATRIX_SIZE];
        for j in 0..(MATRIX_SIZE * MATRIX_SIZE) {
            block[j] = bytes[i * (MATRIX_SIZE * MATRIX_SIZE) + j];
        }
        block = decrypt_block_aes128(&expanded_key, &block);
        for j in 0..MATRIX_SIZE * MATRIX_SIZE {
            result[i * MATRIX_SIZE * MATRIX_SIZE + j] = block[j];
        }
    }

    return result;
}

/// # Description
///  function takes an expanded key and a block of 16 bytes and returns a decrypted block cloned off bytes
/// # Arguments
///
/// * `expanded_key`: expended key from the with key_schedule_aes128 that was called with the original key
/// * `bytes`: block to decrypt
///
/// returns: [u8; 16]
///

fn decrypt_block_aes128(
    expanded_key: &[[u8; MATRIX_SIZE]; MATRIX_SIZE * ROUND_COUNT],
    bytes: &[u8; MATRIX_SIZE * MATRIX_SIZE],
) -> [u8; MATRIX_SIZE * MATRIX_SIZE] {
    let mut result = [0u8; MATRIX_SIZE * MATRIX_SIZE];

    let mut state = [[0u8; MATRIX_SIZE]; MATRIX_SIZE];
    for i in 0..16 {
        state[i % MATRIX_SIZE][i / MATRIX_SIZE] = bytes[i];
    }

    add_round_key(
        &mut state,
        &clone_into_array(
            &expanded_key[MATRIX_SIZE * (ROUND_COUNT - 1)..MATRIX_SIZE * ROUND_COUNT],
        ),
    );
    inverse_shift_rows(&mut state);
    inverse_substitute_bytes(&mut state);

    for i in (1..ROUND_COUNT - 1).rev() {
        add_round_key(
            &mut state,
            &clone_into_array(&expanded_key[i * MATRIX_SIZE..(i + 1) * MATRIX_SIZE]),
        );
        mix_columns(&mut state, false);
        inverse_shift_rows(&mut state);
        inverse_substitute_bytes(&mut state);
    }

    add_round_key(&mut state, &clone_into_array(&expanded_key[0..MATRIX_SIZE]));

    for i in 0..MATRIX_SIZE {
        for j in 0..MATRIX_SIZE {
            result[MATRIX_SIZE * j + i] = state[i][j]
        }
    }

    return result;
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_inverse_shift_rows() {
        let mut matrix1 = [[0, 1, 2, 3], [0, 1, 2, 3], [0, 1, 2, 3], [0, 1, 2, 3]];
        let mut matrix2 = matrix1.clone();

        mix_columns(&mut matrix1, false);

        assert_eq!(matrix2, matrix1)
    }

    #[test]
    fn test_aes128() {
        let text = "yellowbanana1234".as_bytes();
        let key: [u8; 16] = [
            0x2b, 0x7e, 0x15, 0x16, 0x28, 0xae, 0xd2, 0xa6, 0xab, 0xf7, 0x15, 0x88, 0x09, 0xcf,
            0x4f, 0x3c,
        ];

        let new_text = encrypt_aes128(&key, text);

        let new_text = decrypt_aes128(&key, &new_text);

        assert_eq!(new_text, text.to_owned())
    }
}
