[Package Summary](https://docs.oracle.com/en/java/javase/21/docs/api/java.base/java/util/stream/package-summary.html)

- [BiConsumer](https://docs.oracle.com/en/java/javase/21/docs/api/java.base/java/util/function/BiConsumer.html)

- [BinaryOperator](https://docs.oracle.com/en/java/javase/21/docs/api/java.base/java/util/function/BinaryOperator.html)

- [Collector](https://docs.oracle.com/en/java/javase/21/docs/api/java.base/java/util/stream/Collector.html)

- [Consumer](https://docs.oracle.com/en/java/javase/21/docs/api/java.base/java/util/function/Consumer.html)

  - [DoubleConsumer](https://docs.oracle.com/en/java/javase/21/docs/api/java.base/java/util/function/DoubleConsumer.html)
  - [IntConsumer](https://docs.oracle.com/en/java/javase/21/docs/api/java.base/java/util/function/IntConsumer.html)
  - [LongConsumer](https://docs.oracle.com/en/java/javase/21/docs/api/java.base/java/util/function/LongConsumer.html)

- [Function](https://docs.oracle.com/en/java/javase/21/docs/api/java.base/java/util/function/Function.html)

  - [ToDoubleFunction](https://docs.oracle.com/en/java/javase/21/docs/api/java.base/java/util/function/ToDoubleFunction.html)
  - [ToIntFunction](https://docs.oracle.com/en/java/javase/21/docs/api/java.base/java/util/function/ToIntFunction.html)
  - [ToLongFunction](https://docs.oracle.com/en/java/javase/21/docs/api/java.base/java/util/function/ToLongFunction.html)
  - [BiFunction](https://docs.oracle.com/en/java/javase/21/docs/api/java.base/java/util/function/BiFunction.html)
  - [IntFunction](https://docs.oracle.com/en/java/javase/21/docs/api/java.base/java/util/function/IntFunction.html)

- [Optional](https://docs.oracle.com/en/java/javase/21/docs/api/java.base/java/util/Optional.html)

  - [NoSuchElementException](https://docs.oracle.com/en/java/javase/21/docs/api/java.base/java/util/NoSuchElementException.html)

- [Predicate](https://docs.oracle.com/en/java/javase/21/docs/api/java.base/java/util/function/Predicate.html)

- [Stream](https://docs.oracle.com/en/java/javase/21/docs/api/java.base/java/util/stream/Stream.html)

  - [DoubleStream](https://docs.oracle.com/en/java/javase/21/docs/api/java.base/java/util/stream/DoubleStream.html)
  - [IntStream](https://docs.oracle.com/en/java/javase/21/docs/api/java.base/java/util/stream/IntStream.html)
  - [LongStream](https://docs.oracle.com/en/java/javase/21/docs/api/java.base/java/util/stream/LongStream.html)

- [Supplier](https://docs.oracle.com/en/java/javase/21/docs/api/java.base/java/util/function/Supplier.html)

- [UnaryOperator](https://docs.oracle.com/en/java/javase/21/docs/api/java.base/java/util/function/UnaryOperator.html)

# Implementations

### Optional\<T\> Stream.$next()

optional doesn't really work cause it could be null

do I throw an exception for the iteration to stop?

do I have a special magic value for None that's not null? u32::MAX? 1?

how do I return an option from my function? do I really need to allocate a whole object for it?
