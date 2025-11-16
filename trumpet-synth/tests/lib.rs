use common::debouncer::Debouncer;

#[test]
fn debounce_test() {
    let mut debouncer = Debouncer::new(0);

    let states = [false, true, false, false, false];

    for &state in states.iter() {
        debouncer.update(state);
        println!("{:?}", debouncer.is_high());
        println!("{:?}", debouncer,);
        println!("{:?}\n", debouncer.is_high());
    }
}
