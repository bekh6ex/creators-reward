use argon2::password_hash::SaltString;
use argon2::{Algorithm, Params, PasswordHasher, Version};
use criterion::{Criterion, criterion_group, criterion_main};
use uuid::Uuid;

#[expect(
    clippy::expect_used,
    reason = "benchmark setup with valid argon2 parameters"
)]
fn argon2_benchmark(c: &mut Criterion) {
    // let salt = Uuid::new_v4();
    let salt = SaltString::encode_b64(Uuid::new_v4().as_bytes())
        .expect("UUID bytes encode as an argon2 salt");

    c.bench_function("argon2 - default", |b| {
        let params = Params::default();

        b.iter(|| {
            let result = argon2::Argon2::new_with_secret(
                std::hint::black_box("some very secret thing".as_bytes()),
                Algorithm::Argon2id,
                Version::V0x13,
                params.clone(),
            )
            .expect("valid argon2 parameters");
            result
                .hash_password(
                    std::hint::black_box("some password here".as_bytes()),
                    &salt,
                )
                .expect("valid password and salt")
                .hash
                .expect("hash produced");
        })
    });
}

criterion_group!(benches, argon2_benchmark);
criterion_main!(benches);
