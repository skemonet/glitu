//! Code imported in the slides (`<<< src/total.rs#sum`)

// #region sum
pub fn total(v: &[i32]) -> i32 {
    v.iter().sum()
}
// #endregion

fn main() {
    println!("{}", total(&[1, 2, 3]));
}
