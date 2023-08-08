
pub fn version_compare(v1: &str, v2: &str) -> std::cmp::Ordering {
    let v1_parts: Vec<u32> = v1
        .split('.')
        .map(|s| s.parse::<u32>().unwrap())
        .collect();

    let v2_parts: Vec<u32> = v2
        .split('.') 
        .map(|s| s.parse::<u32>().unwrap()) 
        .collect();

    for (v1_part, v2_part) in v1_parts.iter().zip(v2_parts.iter()) {
        match v1_part.cmp(&v2_part) {
            std::cmp::Ordering::Equal => {}
            other => return other,
        }
    }

    v1_parts.len().cmp(&v2_parts.len()) 
}