# HyperBuf
A dynamic and highly optimized buffer with atomic locking mechanisms and asynchronous memory management

For memory retrieval, instead of spin-waiting and blocking the thread, the system uses an asynchronous model for treating the data internally as any type. There are three ways to interact with the data, and it is up to the programmer to make the wisest decisions:

1. Direct treatement of the system as a u8 buffer, or;

2. Asynchronous casting of type to an immutable yet readable version (via ReadVisitors), or;

3. Asynchronous casting of type to a mutable thus writable version (via WriteVisitor's)

The rule for consistency is simple: if you choose to treat the type as a buffer, you should NOT use Write/Read Visitors. 

When you use a Writevisitor, you should specify the amount of bytes you plan on writing when calling visit(). If you don't plan on making the type grow, you can simply enter None.
