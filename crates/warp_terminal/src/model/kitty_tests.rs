use std::fs;

use super::*;

/// Puts `data` in a new shared-memory object `name`, as a program sending a `t=s` image does.
#[cfg(unix)]
fn create_shared_memory(name: &str, data: &[u8]) {
    use std::num::NonZero;

    use nix::fcntl::OFlag;
    use nix::sys::mman::{MapFlags, ProtFlags, mmap, munmap, shm_open};
    use nix::sys::stat::Mode;

    let fd = shm_open(
        name,
        OFlag::O_CREAT | OFlag::O_EXCL | OFlag::O_RDWR,
        Mode::S_IRUSR | Mode::S_IWUSR,
    )
    .unwrap();
    nix::unistd::ftruncate(fd, data.len() as i64).unwrap();
    let size = NonZero::new(data.len()).unwrap();
    unsafe {
        let ptr = mmap(
            None,
            size,
            ProtFlags::PROT_READ | ProtFlags::PROT_WRITE,
            MapFlags::MAP_SHARED,
            fd,
            0,
        )
        .unwrap();
        std::ptr::copy_nonoverlapping(data.as_ptr(), ptr as *mut u8, data.len());
        munmap(ptr, data.len()).unwrap();
    }
    nix::unistd::close(fd).unwrap();
}

#[cfg(unix)]
#[test]
fn test_read_shared_memory_closes_what_it_opens() {
    let open_fds = || fs::read_dir("/dev/fd").unwrap().count();
    let control_data = parse_kitty_control_data(b"f=32,s=2,v=1,t=s");
    let before = open_fds();

    for n in 0..500 {
        let name = format!("/wkt-{}-{n}", std::process::id());
        create_shared_memory(&name, &[7; 8]);
        let data = read_shared_memory(control_data.clone(), name.into_bytes()).unwrap();
        assert_eq!(data, [7; 8]);
    }

    // A read that kept its object open would leave 500 more; other tests may hold a few.
    assert!(open_fds() < before + 100);
}
