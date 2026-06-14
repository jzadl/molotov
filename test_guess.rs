pub fn get_libs_dir() -> String {
    return "libs".to_string();
}

pub fn remove(repo: String) -> () {
    let libs_dir = get_libs_dir();
    let target_path = libs_dir + &"/".to_string() + &repo;
    println!("{}", repo);
}

