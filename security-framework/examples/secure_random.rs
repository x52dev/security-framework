use security_framework::random::SecRandom;

fn main() {
    let mut buf = [0; 32];

    let rng = SecRandom::default();
    rng.copy_bytes(&mut buf).unwrap();

    println!("{}", hex::encode(buf));
}
