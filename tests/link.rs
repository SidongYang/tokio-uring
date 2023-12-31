use std::{task::Context, io::Write, os::unix::prelude::OpenOptionsExt};

use futures_util::{future::{Either, BoxFuture, join_all}, Future, FutureExt};
use io_uring::squeue::Flags;
use tempfile::NamedTempFile;
use tokio_uring::{fs::File, buf::IoBufMut, BufResult};
use std::task::Poll;

struct UnsafeBuffer {
	addr : *mut u8
}

unsafe impl tokio_uring::buf::IoBuf for UnsafeBuffer {
    fn stable_ptr(&self) -> *const u8 {
        self.addr
    }

    fn bytes_init(&self) -> usize {
        std::mem::size_of::<u8>()
    }

    fn bytes_total(&self) -> usize {
        std::mem::size_of::<u8>()
    }
}

unsafe impl IoBufMut for UnsafeBuffer {
    fn stable_mut_ptr(&mut self) -> *mut u8 {
		self.addr
    }
    unsafe fn set_init(&mut self, _pos: usize) {}
}

fn tempfile() -> NamedTempFile {
    NamedTempFile::new().unwrap()
}

#[test]
fn multiple_write() {
    tokio_uring::start(async {
        let tempfile = tempfile();
		let file = std::fs::OpenOptions::new().read(true).write(true)
			.custom_flags(libc::O_NONBLOCK)
			.open(tempfile.path()).unwrap();
		let file = tokio_uring::fs::File::from_std(file);
		let mut tasks = Vec::new();

		for i in 0..20 {
			let buf = i.to_string();
			// let write_task = file.write_at(buf.into_bytes(), 0).submit();
			let write_task = file.write_at_with_flags(buf.into_bytes(), 0, Flags::IO_LINK).submit();			
			let task = tokio_uring::spawn(write_task);
			tasks.push(task);
		}

		

        let file = File::open(tempfile.path()).await.unwrap();		
		let _ = futures_util::future::join_all(tasks).await;

		let buf = vec![0u8, 10];
		let (result, buf) = file.read_at(buf, 0).await;
		result.unwrap();
		let str = String::from_utf8(buf).unwrap();
		assert_eq!(&str, "19");
    });
}


#[test]
fn multiple_read() {
    tokio_uring::start(async {
        let tempfile = tempfile();
		let file = std::fs::OpenOptions::new().read(true).write(true)
			.custom_flags(libc::O_NONBLOCK)
			.open(tempfile.path()).unwrap();
		let file = tokio_uring::fs::File::from_std(file);
		let mut tasks = Vec::new();

		for i in 0..20 {
			let buf = i.to_string();
			let read_task = file.unsubmitted_read_at_with_flags(buf.into_bytes(), 0, Flags::IO_LINK).submit();
			let task = tokio_uring::spawn(read_task);
			tasks.push(task);
		}

        let file = File::open(tempfile.path()).await.unwrap();		
		let _ = futures_util::future::join_all(tasks).await;

		let buf = vec![0u8, 10];
		let (result, buf) = file.read_at(buf, 0).await;
		result.unwrap();
		let str = String::from_utf8(buf).unwrap();
		assert_eq!(&str, "19");
    });
}


#[test]
fn mix_read_write() {
    tokio_uring::start(async {
        let tempfile = tempfile();
		let file = std::fs::OpenOptions::new().read(true).write(true)
			.custom_flags(libc::O_NONBLOCK)
			.open(tempfile.path()).unwrap();
		let file = tokio_uring::fs::File::from_std(file);

		let mut buffer = 0u8;

		let mut tasks = Vec::new();
		for i in 0..100 {
			let buf = UnsafeBuffer { addr: std::ptr::addr_of_mut!(buffer), };
			let write_task = file.write_at_with_flags(buf, 0, Flags::IO_LINK).submit();
			tasks.push(tokio_uring::spawn(write_task));

			let buf = UnsafeBuffer { addr: std::ptr::addr_of_mut!(buffer), };
			let read_task = file.unsubmitted_read_at_with_flags(buf, 0, Flags::IO_LINK).submit();
			tasks.push(tokio_uring::spawn(read_task));
		}

		
		
    });
}
