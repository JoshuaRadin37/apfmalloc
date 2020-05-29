#[macro_use] pub mod macros;
mod size_classes;
mod mem_info;
mod allocation_data;

#[macro_use]
extern crate bitfield;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
