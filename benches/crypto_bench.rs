use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};

// Benchmark encrypt/decrypt at various payload sizes
fn bench_encryption(c: &mut Criterion) {
    // Since envsafe is a binary crate, we inline the crypto operations directly
    use aes_gcm::{
        aead::{Aead, KeyInit},
        Aes256Gcm, Nonce,
    };
    use rand::RngCore;

    let mut key_bytes = vec![0u8; 32];
    rand::thread_rng().fill_bytes(&mut key_bytes);

    let mut group = c.benchmark_group("encryption");
    for size in [64, 256, 1024, 4096, 16384].iter() {
        let plaintext = vec![0u8; *size];

        group.bench_with_input(BenchmarkId::new("encrypt", size), size, |b, _| {
            b.iter(|| {
                let cipher = Aes256Gcm::new_from_slice(&key_bytes).unwrap();
                let mut nonce_bytes = [0u8; 12];
                rand::thread_rng().fill_bytes(&mut nonce_bytes);
                let nonce = Nonce::from_slice(&nonce_bytes);
                cipher
                    .encrypt(nonce, black_box(plaintext.as_slice()))
                    .unwrap()
            })
        });
    }
    group.finish();

    let mut group = c.benchmark_group("decryption");
    for size in [64, 256, 1024, 4096, 16384].iter() {
        let plaintext = vec![0u8; *size];
        let cipher = Aes256Gcm::new_from_slice(&key_bytes).unwrap();
        let mut nonce_bytes = [0u8; 12];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);
        let ciphertext = cipher.encrypt(nonce, plaintext.as_slice()).unwrap();

        let mut data = Vec::with_capacity(12 + ciphertext.len());
        data.extend_from_slice(&nonce_bytes);
        data.extend_from_slice(&ciphertext);

        group.bench_with_input(BenchmarkId::new("decrypt", size), size, |b, _| {
            b.iter(|| {
                let (n, ct) = data.split_at(12);
                let nonce = Nonce::from_slice(n);
                let cipher = Aes256Gcm::new_from_slice(&key_bytes).unwrap();
                cipher.decrypt(nonce, black_box(ct)).unwrap()
            })
        });
    }
    group.finish();
}

fn bench_key_derivation(c: &mut Criterion) {
    use argon2::Argon2;
    use argon2::PasswordHasher;

    c.bench_function("argon2id_derive", |b| {
        b.iter(|| {
            let salt = argon2::password_hash::SaltString::generate(&mut rand::thread_rng());
            let argon2 = Argon2::default();
            let hash = argon2
                .hash_password(black_box(b"test-passphrase"), &salt)
                .unwrap();
            black_box(hash.hash.unwrap().as_bytes().len());
        })
    });
}

criterion_group!(benches, bench_encryption, bench_key_derivation);
criterion_main!(benches);
