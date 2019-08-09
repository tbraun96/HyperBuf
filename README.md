# HyperBuf
A dynamic and highly optimized buffer with atomic locking mechanisms and asynchronous memory management (async/await ready; use nightly!)

HyperBuf is about 1.4x faster than BytesMut, and 3.4x faster than std::vec!

For memory retrieval, instead of spin-waiting and blocking the thread, the system uses an atomically-backed and asynchronous model that has the capacity to treat the data internally as any arbitrary type. This is especially useful for writing a stream of network bytes to a custom Packet type. This is Zerocopy on steroids.

There are three ways to interact with the data, and it is up to the programmer to make the wisest decisions:

1. Direct treatment of the system as a u8 buffer, or;

2. Asynchronous casting of type to an immutable yet readable version (via ReadVisitors), or;

3. Asynchronous casting of type to a mutable thus writable version (via WriteVisitor's)

The rule for consistency is simple: if you choose to treat the type as a buffer, you should NOT use Write/Read Visitors. 

When you use a WriteVisitor, you should specify the amount of bytes you plan on writing when calling visit(). If you don't plan on making the type grow, you can simply enter None.

The buffer is faster than the std::vec, and faster than BytesMut:


```
Vec benches/std vec     time:   [72.469 ns 72.481 ns 72.492 ns]
                        change: [-2.4623% -2.3697% -2.2885%] (p = 0.00 < 0.05)
                        Performance has improved.
Found 5 outliers among 100 measurements (5.00%)
  1 (1.00%) low severe
  3 (3.00%) low mild
  1 (1.00%) high severe


Vec benches/HyperVec    time:   [21.062 ns 21.066 ns 21.069 ns]
                        change: [-1.8435% -0.8284% -0.1495%] (p = 0.03 < 0.05)
                        Change within noise threshold.
Found 6 outliers among 100 measurements (6.00%)
  2 (2.00%) low mild
  2 (2.00%) high mild
  2 (2.00%) high severe


Vec benches/BytesMut    time:   [29.862 ns 29.867 ns 29.871 ns]
                        change: [+5.3932% +5.4225% +5.4517%] (p = 0.00 < 0.05)
                        Performance has regressed.
Found 6 outliers among 100 measurements (6.00%)
  3 (3.00%) high mild
  3 (3.00%) high severe
```
