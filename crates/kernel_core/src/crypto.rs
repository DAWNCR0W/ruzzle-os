/// SHA-256 output size in bytes.
pub const SHA256_OUTPUT_LEN: usize = 32;

const BLOCK_SIZE: usize = 64;

const H0: [u32; 8] = [
    0x6a09e667,
    0xbb67ae85,
    0x3c6ef372,
    0xa54ff53a,
    0x510e527f,
    0x9b05688c,
    0x1f83d9ab,
    0x5be0cd19,
];

const K: [u32; 64] = [
    0x428a2f98,
    0x71374491,
    0xb5c0fbcf,
    0xe9b5dba5,
    0x3956c25b,
    0x59f111f1,
    0x923f82a4,
    0xab1c5ed5,
    0xd807aa98,
    0x12835b01,
    0x243185be,
    0x550c7dc3,
    0x72be5d74,
    0x80deb1fe,
    0x9bdc06a7,
    0xc19bf174,
    0xe49b69c1,
    0xefbe4786,
    0x0fc19dc6,
    0x240ca1cc,
    0x2de92c6f,
    0x4a7484aa,
    0x5cb0a9dc,
    0x76f988da,
    0x983e5152,
    0xa831c66d,
    0xb00327c8,
    0xbf597fc7,
    0xc6e00bf3,
    0xd5a79147,
    0x06ca6351,
    0x14292967,
    0x27b70a85,
    0x2e1b2138,
    0x4d2c6dfc,
    0x53380d13,
    0x650a7354,
    0x766a0abb,
    0x81c2c92e,
    0x92722c85,
    0xa2bfe8a1,
    0xa81a664b,
    0xc24b8b70,
    0xc76c51a3,
    0xd192e819,
    0xd6990624,
    0xf40e3585,
    0x106aa070,
    0x19a4c116,
    0x1e376c08,
    0x2748774c,
    0x34b0bcb5,
    0x391c0cb3,
    0x4ed8aa4a,
    0x5b9cca4f,
    0x682e6ff3,
    0x748f82ee,
    0x78a5636f,
    0x84c87814,
    0x8cc70208,
    0x90befffa,
    0xa4506ceb,
    0xbef9a3f7,
    0xc67178f2,
];

/// Minimal SHA-256 hasher (no_std friendly).
#[derive(Clone)]
pub struct Sha256 {
    state: [u32; 8],
    buffer: [u8; BLOCK_SIZE],
    buffer_len: usize,
    bit_len: u64,
}

impl Sha256 {
    /// Creates a new SHA-256 hasher.
    pub const fn new() -> Self {
        Self {
            state: H0,
            buffer: [0u8; BLOCK_SIZE],
            buffer_len: 0,
            bit_len: 0,
        }
    }

    /// Updates the hash with more data.
    pub fn update(&mut self, data: &[u8]) {
        self.bit_len = self.bit_len.wrapping_add((data.len() as u64) * 8);
        for &byte in data {
            self.buffer[self.buffer_len] = byte;
            self.buffer_len += 1;
            if self.buffer_len == BLOCK_SIZE {
                self.transform();
                self.buffer_len = 0;
            }
        }
    }

    /// Finalizes the hash and returns the digest.
    pub fn finalize(mut self) -> [u8; SHA256_OUTPUT_LEN] {
        let bit_len = self.bit_len;
        self.buffer[self.buffer_len] = 0x80;
        self.buffer_len += 1;

        if self.buffer_len > 56 {
            for byte in self.buffer[self.buffer_len..].iter_mut() {
                *byte = 0;
            }
            self.transform();
            self.buffer_len = 0;
        }

        for byte in self.buffer[self.buffer_len..56].iter_mut() {
            *byte = 0;
        }

        self.buffer[56..].copy_from_slice(&bit_len.to_be_bytes());
        self.transform();

        let mut out = [0u8; SHA256_OUTPUT_LEN];
        for (chunk, word) in out.chunks_exact_mut(4).zip(self.state.iter()) {
            chunk.copy_from_slice(&word.to_be_bytes());
        }
        out
    }

    fn transform(&mut self) {
        let mut w = [0u32; 64];
        for (idx, chunk) in self.buffer.chunks_exact(4).take(16).enumerate() {
            w[idx] = u32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        }
        for idx in 16..64 {
            let s0 = small_sigma0(w[idx - 15]);
            let s1 = small_sigma1(w[idx - 2]);
            w[idx] = w[idx - 16]
                .wrapping_add(s0)
                .wrapping_add(w[idx - 7])
                .wrapping_add(s1);
        }

        let mut a = self.state[0];
        let mut b = self.state[1];
        let mut c = self.state[2];
        let mut d = self.state[3];
        let mut e = self.state[4];
        let mut f = self.state[5];
        let mut g = self.state[6];
        let mut h = self.state[7];

        for idx in 0..64 {
            let t1 = h
                .wrapping_add(big_sigma1(e))
                .wrapping_add(ch(e, f, g))
                .wrapping_add(K[idx])
                .wrapping_add(w[idx]);
            let t2 = big_sigma0(a).wrapping_add(maj(a, b, c));
            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }

        self.state[0] = self.state[0].wrapping_add(a);
        self.state[1] = self.state[1].wrapping_add(b);
        self.state[2] = self.state[2].wrapping_add(c);
        self.state[3] = self.state[3].wrapping_add(d);
        self.state[4] = self.state[4].wrapping_add(e);
        self.state[5] = self.state[5].wrapping_add(f);
        self.state[6] = self.state[6].wrapping_add(g);
        self.state[7] = self.state[7].wrapping_add(h);
    }
}

/// Computes a SHA-256 digest in one shot.
pub fn sha256(data: &[u8]) -> [u8; SHA256_OUTPUT_LEN] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.finalize()
}

/// Computes an HMAC-SHA256 digest for a single buffer.
pub fn hmac_sha256(key: &[u8], data: &[u8]) -> [u8; SHA256_OUTPUT_LEN] {
    hmac_sha256_parts(key, &[data])
}

/// Computes an HMAC-SHA256 digest for multiple buffers.
pub fn hmac_sha256_parts(key: &[u8], parts: &[&[u8]]) -> [u8; SHA256_OUTPUT_LEN] {
    let key_block = normalize_key(key);
    let mut o_key_pad = [0u8; BLOCK_SIZE];
    let mut i_key_pad = [0u8; BLOCK_SIZE];
    for idx in 0..BLOCK_SIZE {
        o_key_pad[idx] = key_block[idx] ^ 0x5c;
        i_key_pad[idx] = key_block[idx] ^ 0x36;
    }

    let mut inner = Sha256::new();
    inner.update(&i_key_pad);
    for part in parts {
        inner.update(part);
    }
    let inner_hash = inner.finalize();

    let mut outer = Sha256::new();
    outer.update(&o_key_pad);
    outer.update(&inner_hash);
    outer.finalize()
}

fn normalize_key(key: &[u8]) -> [u8; BLOCK_SIZE] {
    let mut block = [0u8; BLOCK_SIZE];
    if key.len() > BLOCK_SIZE {
        let hashed = sha256(key);
        block[..SHA256_OUTPUT_LEN].copy_from_slice(&hashed);
    } else if !key.is_empty() {
        block[..key.len()].copy_from_slice(key);
    }
    block
}

#[inline]
fn rotr(value: u32, amount: u32) -> u32 {
    (value >> amount) | (value << (32 - amount))
}

#[inline]
fn ch(x: u32, y: u32, z: u32) -> u32 {
    (x & y) ^ (!x & z)
}

#[inline]
fn maj(x: u32, y: u32, z: u32) -> u32 {
    (x & y) ^ (x & z) ^ (y & z)
}

#[inline]
fn big_sigma0(x: u32) -> u32 {
    rotr(x, 2) ^ rotr(x, 13) ^ rotr(x, 22)
}

#[inline]
fn big_sigma1(x: u32) -> u32 {
    rotr(x, 6) ^ rotr(x, 11) ^ rotr(x, 25)
}

#[inline]
fn small_sigma0(x: u32) -> u32 {
    rotr(x, 7) ^ rotr(x, 18) ^ (x >> 3)
}

#[inline]
fn small_sigma1(x: u32) -> u32 {
    rotr(x, 17) ^ rotr(x, 19) ^ (x >> 10)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::string::String;

    fn hex(bytes: &[u8]) -> String {
        let mut out = String::new();
        for byte in bytes {
            out.push_str(&format!("{:02x}", byte));
        }
        out
    }

    #[test]
    fn sha256_empty_matches_vector() {
        let digest = sha256(b"");
        assert_eq!(
            hex(&digest),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn sha256_abc_matches_vector() {
        let mut hasher = Sha256::new();
        hasher.update(b"a");
        hasher.update(b"bc");
        let digest = hasher.finalize();
        assert_eq!(
            hex(&digest),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn sha256_padding_requires_extra_block() {
        let payload = [b'a'; 56];
        let digest = sha256(&payload);
        assert_eq!(
            hex(&digest),
            "b35439a4ac6f0948b6d6f9e3c6af0f5f590ce20f1bde7090ef7970686ec6738a"
        );
    }

    #[test]
    fn hmac_sha256_vector_short_key() {
        let key = [0x0b_u8; 20];
        let digest = hmac_sha256(&key, b"Hi There");
        assert_eq!(
            hex(&digest),
            "b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7"
        );
    }

    #[test]
    fn hmac_sha256_vector_jefe() {
        let digest = hmac_sha256(b"Jefe", b"what do ya want for nothing?");
        assert_eq!(
            hex(&digest),
            "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843"
        );
    }

    #[test]
    fn hmac_sha256_vector_long_key() {
        let key = [0xaa_u8; 131];
        let digest =
            hmac_sha256(&key, b"Test Using Larger Than Block-Size Key - Hash Key First");
        assert_eq!(
            hex(&digest),
            "60e431591ee0b67f0d8a26aacbf5b77f8e0bc6213728c5140546040f0ee37f54"
        );
    }

    #[test]
    fn hmac_sha256_parts_matches_single_buffer() {
        let key = b"ruzzle";
        let digest_parts = hmac_sha256_parts(key, &[b"hello ", b"world"]);
        let digest = hmac_sha256(key, b"hello world");
        assert_eq!(digest_parts, digest);
    }

    #[test]
    fn hmac_sha256_accepts_empty_key() {
        let digest = hmac_sha256(b"", b"data");
        assert_eq!(
            hex(&digest),
            "e528c4d99e6177f5841f712a143b90843299a4aa181a06501422d9ca862bd2a5"
        );
    }
}
