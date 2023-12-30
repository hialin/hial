use std::time::Instant;

pub fn perftests(alloc_count: usize) {
    {
        let size = 1024 * 1024 * 1024;
        let mut x = Vec::with_capacity(size);
        for i in 0..size {
            x.push(i);
        }
        for i in 0..size {
            println!("bigalloc: {}", x.iter().sum::<usize>());
        }
    }

    let mut sizes = vec![0; alloc_count];
    let mut allocations = Vec::<Vec<u8>>::new();
    {
        for i in 0..alloc_count {
            allocations.push(Vec::new());
        }
    }

    for i in 0..10 {
        random_sizes(&mut sizes, 8 * i + 1);
        make_allocations(&sizes, &mut allocations);
        use_allocations(&mut allocations);
        measure(alloc_count, &sizes, &mut allocations);
    }
}

fn random_sizes(sizes: &mut [usize], max_size_in_words: usize) {
    use rand::Rng;
    for i in 0..sizes.len() {
        let v = rand::thread_rng().gen_range(0..max_size_in_words);
        sizes[i] = 8 * (v + 1);
    }
}

fn make_allocations(sizes: &[usize], allocations: &mut [Vec<u8>]) {
    for (i, size) in sizes.iter().enumerate() {
        // let x = vec![*size as u8; *size];
        let x = Vec::with_capacity(*size);
        allocations[i] = x;
    }
}

fn use_allocations(allocations: &mut [Vec<u8>]) {
    for (i, a) in allocations.iter_mut().enumerate() {
        a.push(1_u8);
        // a[0] = i as u8;
    }

    let mut sum: usize = 0;
    for (i, a) in allocations.iter().enumerate() {
        sum += a[0] as usize;
    }
    println!("---\nsum = {}", sum);
}

fn measure(alloc_count: usize, sizes: &[usize], allocations: &mut [Vec<u8>]) {
    println!(
        "sizes: {:?}\ntotal allocated size: {}MB\nmaximum size: {}B",
        &sizes[0..100],
        sizes.iter().sum::<usize>() / 1024 / 1024,
        *sizes.iter().max().unwrap(),
    );

    let start = Instant::now();
    {
        for (i, size) in sizes.iter().enumerate() {
            let x = vec![*size as u8; *size];
            allocations[i] = x;
        }
    }
    let duration = Instant::now() - start;

    println!(
        "performed {} allocations in {} seconds",
        alloc_count,
        duration.as_secs()
    );

    if alloc_count > 0 {
        println!(
            "{} ns per allocation",
            duration.as_nanos() as f64 / (alloc_count as f64)
        );
    }
    println!(
        "{} ns per byte",
        duration.as_nanos() as f64 / (sizes.iter().sum::<usize>() as f64)
    );
}
