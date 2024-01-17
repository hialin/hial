use crate::base::*;
use crate::interpretations::*;

#[test]
fn test_cell_sizes() -> Res<()> {
    println!("size of file cell: {}", std::mem::size_of::<fs::Cell>());
    println!("size of http cell: {}", std::mem::size_of::<http::Cell>());
    println!("size of json cell: {}", std::mem::size_of::<json::Cell>());
    println!(
        "size of ownvalue cell: {}",
        std::mem::size_of::<ownvalue::Cell>()
    );
    println!("size of path cell: {}", std::mem::size_of::<path::Cell>());
    println!("size of toml cell: {}", std::mem::size_of::<toml::Cell>());
    println!(
        "size of treesitter cell: {}",
        std::mem::size_of::<treesitter::Cell>()
    );
    println!("size of url cell: {}", std::mem::size_of::<url::Cell>());
    println!("size of xml cell: {}", std::mem::size_of::<xml::Cell>());
    println!("size of yaml cell: {}", std::mem::size_of::<yaml::Cell>());
    println!("size of cell: {}", std::mem::size_of::<Cell>());

    assert!(std::mem::size_of::<Cell>() <= 6 * 8);
    Ok(())
}
