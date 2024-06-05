pub trait Split<T> {
    fn rsplit_once(&self, element: T) -> Option<(&[T], &[T])>
    where
        T: PartialEq;
}

impl<T> Split<T> for [T] {
    fn rsplit_once(&self, element: T) -> Option<(&[T], &[T])>
    where
        T: PartialEq,
    {
        let elements: Vec<_> = self.rsplitn(2, |e| e == &element).collect();
        let elements: [&[T]; 2] = elements.try_into().ok()?;
        let (right, left) = elements.into();

        Some((left, right))
    }
}

impl<T> Split<T> for &[T] {
    fn rsplit_once(&self, element: T) -> Option<(&[T], &[T])>
    where
        T: PartialEq,
    {
        Split::rsplit_once(*self, element)
    }
}

impl<T> Split<T> for Vec<T> {
    fn rsplit_once(&self, element: T) -> Option<(&[T], &[T])>
    where
        T: PartialEq,
    {
        Split::rsplit_once(self.as_slice(), element)
    }
}
