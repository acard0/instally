

pub fn version_compare(v1: &str, v2: &str) -> std::cmp::Ordering {
    let v1_int = v1.replace(".", "")
        .parse::<u32>()
        .unwrap();

    let v2_int = v2.replace(".", "")
        .parse::<u32>()
        .unwrap();

    v1_int.cmp(&v2_int)
}