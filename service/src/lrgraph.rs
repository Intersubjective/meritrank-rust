use left_right::{Absorb, ReadHandle, WriteHandle};

// First, define an operational log type.
// For most real-world use-cases, this will be an `enum`, but we'll keep it simple:
pub struct CounterAddOp(i32);

impl CounterAddOp {
    pub fn new() -> CounterAddOp {
        CounterAddOp(0)
    }
}

// Then, implement the unsafe `Absorb` trait for your data structure type,
// and provide the oplog type as the generic argument.
// You can read this as "`i32` can absorb changes of type `CounterAddOp`".
impl Absorb<CounterAddOp> for i32 {
    // See the documentation of `Absorb::absorb_first`.
    //
    // Essentially, this is where you define what applying
    // the oplog type to the datastructure does.
    fn absorb_first(&mut self, operation: &mut CounterAddOp, _: &Self) {
        *self += operation.0;
    }

    // See the documentation of `Absorb::absorb_second`.
    //
    // This may or may not be the same as `absorb_first`,
    // depending on whether or not you de-duplicate values
    // across the two copies of your data structure.
    fn absorb_second(&mut self, operation: CounterAddOp, _: &Self) {
        *self += operation.0;
    }

    // See the documentation of `Absorb::drop_first`.
    fn drop_first(self: Box<Self>) {}

    fn sync_with(&mut self, first: &Self) {
        *self = *first
    }
}

// You will likely want to embed these handles in your own types so that you can
// provide more ergonomic methods for performing operations on your type.
pub struct Counter(WriteHandle<i32, CounterAddOp>);
impl Counter {
    pub fn new(write: WriteHandle<i32, CounterAddOp>) -> Self {
        Counter(write)
    }
    
    // The methods on you write handle type will likely all just add to the operational log.
    pub fn add(&mut self, i: i32) {
        self.0.append(CounterAddOp(i));
    }

    // You should also provide a method for exposing the results of any pending operations.
    //
    // Until this is called, any writes made since the last call to `publish` will not be
    // visible to readers. See `WriteHandle::publish` for more details. Make sure to call
    // this out in _your_ documentation as well, so that your users will be aware of this
    // "weird" behavior.
    pub fn publish(&mut self) {
        self.0.publish();
    }
}

// Similarly, for reads:
#[derive(Clone)]
pub struct CountReader(ReadHandle<i32>);
impl CountReader {
        pub fn new(read: ReadHandle<i32>) -> Self {
        CountReader(read)
    }
    
    
    pub fn get(&self) -> i32 {
        // The `ReadHandle` itself does not allow you to access the underlying data.
        // Instead, you must first "enter" the data structure. This is similar to
        // taking a `Mutex`, except that no lock is actually taken. When you enter,
        // you are given back a guard, which gives you shared access (through the
        // `Deref` trait) to the "read copy" of the data structure.
        //
        // Note that `enter` may yield `None`, which implies that the `WriteHandle`
        // was dropped, and took the backing data down with it.
        //
        // Note also that for as long as the guard lives, a writer that tries to
        // call `WriteHandle::publish` will be blocked from making progress.
        self.0.enter().map(|guard| *guard).unwrap_or(0)
    }
}