fn main() {
    println!("Hello, world!");
}

fn bar(_x: &mut i32) {}

fn e502(a: &mut i32) {
    let y = &a; 
    //bar(a);
    // newlineabcdefghijk
    // abc 
    // bar(a); 
    println!("{}", y);
}  