pub trait Stack<T>: Sized {
    fn pushd<K: Stackable<T>>(&mut self, value: K) {
        K::push(self, value);
    }
    fn push_one(&mut self, value: T);
    fn popd<K: Stackable<T>>(&mut self) -> Option<K> {
        K::pop(self)
    }
    fn pop_one(&mut self) -> Option<T>;
}

impl<T> Stack<T> for Vec<T> {
    fn push_one(&mut self, value: T) {
        self.push(value);
    }

    fn pop_one(&mut self) -> Option<T> {
        self.pop()
    }
}

pub trait Stackable<T>: Sized {
    fn push(stack: &mut impl Stack<T>, value: Self);
    fn pop(stack: &mut impl Stack<T>) -> Option<Self>;
}

impl<T> Stackable<T> for T {
    fn pop(stack: &mut impl Stack<T>) -> Option<Self> {
        stack.pop_one()
    }

    fn push(stack: &mut impl Stack<T>, value: Self) {
        stack.pushd(value);
    }
}

impl Stackable<u32> for i32 {
    fn pop(stack: &mut impl Stack<u32>) -> Option<Self> {
        stack.pop_one().map(|i| i as Self)
    }

    fn push(stack: &mut impl Stack<u32>, value: Self) {
        stack.push_one(value as u32);
    }
}

impl Stackable<u32> for f32 {
    fn pop(stack: &mut impl Stack<u32>) -> Option<Self> {
        stack.pop_one().map(Self::from_bits)
    }

    fn push(stack: &mut impl Stack<u32>, value: Self) {
        stack.pushd(value.to_bits());
    }
}

impl Stackable<u32> for u64 {
    fn pop(stack: &mut impl Stack<u32>) -> Option<Self> {
        let lower = stack.pop_one()?;
        let upper = stack.pop_one()?;
        Some((upper as Self) << 32 | lower as Self)
    }

    fn push(stack: &mut impl Stack<u32>, value: Self) {
        let lower = (value & 0xFFFF_FFFF) as u32;
        let upper = (value >> 32) as u32;
        stack.push_one(upper);
        stack.push_one(lower);
    }
}

impl Stackable<u32> for f64 {
    fn pop(stack: &mut impl Stack<u32>) -> Option<Self> {
        stack.popd::<u64>().map(Self::from_bits)
    }

    fn push(stack: &mut impl Stack<u32>, value: Self) {
        stack.pushd(value.to_bits());
    }
}

impl Stackable<u32> for i64 {
    fn pop(stack: &mut impl Stack<u32>) -> Option<Self> {
        stack.popd::<u64>().map(|i| i as Self)
    }

    fn push(stack: &mut impl Stack<u32>, value: Self) {
        stack.pushd(value as u64);
    }
}
