pub fn main() {
    // This statement prevents cargo from complaining about an unknown (to it)
    // cfg for fuzzing.
    println!("cargo::rustc-check-cfg=cfg(fuzzing)");
}
