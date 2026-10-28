//! Exemple de code importé dans les slides (`<<< src/total.rs#somme`)

// #region somme
pub fn total(v: &[i32]) -> i32 {
    v.iter().sum()
}
// #endregion

fn main() {
    println!("{}", total(&[1, 2, 3]));
}
