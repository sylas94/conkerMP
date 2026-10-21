//! The extractor's heap: dlmalloc (std's own on wasm), but its memory grown [`STEP`] at a time. Std's
//! grows the wasm memory by as little as one 64 KiB page whenever it runs out, and V8 (Chrome, Node)
//! on Windows takes time in proportion to the memory's size for each grow: the pack's ~700 MB peak
//! came in some 11,000 grows, 28 of the extractor's 34 s. In 32 MiB steps it is a few dozen.

use core::alloc::{GlobalAlloc, Layout};
use core::arch::wasm32::memory_grow;
use core::cell::UnsafeCell;
use core::ptr;
use dlmalloc::{Allocator, Dlmalloc};

/// The least the memory grows by.
const STEP: usize = 32 << 20;
/// Wasm's page.
const PAGE: usize = 64 << 10;

struct Grow;

unsafe impl Allocator for Grow {
    fn alloc(&self, size: usize) -> (*mut u8, usize, u32) {
        // A whole step if the browser gives one, else just what was asked.
        for size in [size.max(STEP), size] {
            let pages = size.div_ceil(PAGE);
            let prev = memory_grow(0, pages);
            if prev != usize::MAX {
                return ((prev * PAGE) as *mut u8, pages * PAGE, 0);
            }
        }
        (ptr::null_mut(), 0, 0)
    }

    fn remap(&self, _ptr: *mut u8, _old: usize, _new: usize, _can_move: bool) -> *mut u8 {
        ptr::null_mut()
    }

    fn free_part(&self, _ptr: *mut u8, _old: usize, _new: usize) -> bool {
        false
    }

    fn free(&self, _ptr: *mut u8, _size: usize) -> bool {
        false
    }

    fn can_release_part(&self, _flags: u32) -> bool {
        false
    }

    fn allocates_zeros(&self) -> bool {
        true
    }

    fn page_size(&self) -> usize {
        PAGE
    }
}

struct Heap(UnsafeCell<Dlmalloc<Grow>>);

// The extractor runs on one thread (wasm32 without atomics): nothing else ever holds the heap.
unsafe impl Sync for Heap {}

#[global_allocator]
static HEAP: Heap = Heap(UnsafeCell::new(Dlmalloc::new_with_allocator(Grow)));

unsafe impl GlobalAlloc for Heap {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        unsafe { (*self.0.get()).malloc(layout.size(), layout.align()) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { (*self.0.get()).free(ptr, layout.size(), layout.align()) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        unsafe { (*self.0.get()).calloc(layout.size(), layout.align()) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        unsafe { (*self.0.get()).realloc(ptr, layout.size(), layout.align(), new_size) }
    }
}
