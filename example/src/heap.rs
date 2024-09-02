use embedded_alloc::Heap as EmbeddedAllocHeap;

#[global_allocator]
static HEAP: EmbeddedAllocHeap = EmbeddedAllocHeap::empty();

const HEAP_SIZE: usize = 65_536;

pub struct Heap {}

impl Heap {
    pub fn init() {
        use core::mem::MaybeUninit;

        static mut HEAP_MEM: [MaybeUninit<u8>; HEAP_SIZE] = [MaybeUninit::uninit(); HEAP_SIZE];
        unsafe { HEAP.init(HEAP_MEM.as_ptr() as usize, HEAP_SIZE) }
    }
}
