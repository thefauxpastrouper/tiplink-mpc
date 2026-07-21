fn main() {
    let mut exp = [0u8; 256];
    let mut log = [0u8; 256];
    let mut x: u16 = 1;
    let mut i = 0;
    while i < 255 {
        exp[i] = x as u8;
        log[x as usize] = i as u8;
        x <<= 1;
        if (x & 0x100) != 0 {
            x ^= 0x11D;
        }
        i += 1;
    }
    
    let mut count = 0;
    for e in exp.iter() {
        if *e == 1 { count += 1; }
    }
    println!("Count of 1: {}", count);
}
