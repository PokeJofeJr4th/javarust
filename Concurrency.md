Braindump of requirements related to concurrency and synchronization

- [Synchronization Section](https://docs.oracle.com/javase/specs/jvms/se21/html/jvms-2.html#jvms-2.11.10)

- Monitors

  - [Monitor Enter](https://docs.oracle.com/javase/specs/jvms/se21/html/jvms-6.html#jvms-6.5.monitorenter)
  - [Monitor Exit](https://docs.oracle.com/javase/specs/jvms/se21/html/jvms-6.html#jvms-6.5.monitorexit)
  - [Object methods](https://docs.oracle.com/javase/8/docs/api/java/lang/Object.html)
  - Synchronized Methods

- Threads
  - [Thread Class](https://docs.oracle.com/javase/8/docs/api/java/lang/Thread.html)

### Implementation

global set of Monitor objects (maybe handled by the heap?)

current thread has a collection of MutexGuards it holds

monitorenter -> check if the current thread has a mutex guard on the monitor. Else block on lock.

monitorexit -> remove and drop the lock from the Thread's collection

remember to include this in synchronized method invocation and return as well as exception propagation

what is going on with wait/notify?? AAAA
