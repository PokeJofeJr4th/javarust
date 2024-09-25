## Usage

JVM-RS requires rust to be installed. Then use `cargo run -- -h` to display help text, or keep reading to see usage examples.

The simplest use is `cargo run -- path/to/File.class`. This will follow the relative path to the file and execute its main method, if found. Use another `--` after the command's arguments to pass arguments to the program upon execution: `cargo run -- path/to/File.class -- some arguments here`.

If that file depends on other class files (most common if it is part of a project), add those other class files to the end -- `cargo run -- path/to/File.class path/to/Another.class`. It is also possible to use a text file with one relative path per line to load additional class files. For example, given the contents of `path/to/project.txt` below, the command `cargo run -- path/to/File.class path/to/Another.class` is equivalent to `cargo run -- -p path/to/project.txt`:

```
../File.class
../Another.class
```

### Debugging JVM-RS

To enter debug mode, add `-v`. This will print a very verbose representation of the contents of the class file and each instruction executed. This has extreme consequences for performance and so should be used sparingly. To skip running the class, add `-s`. This can be useful for debugging issues relating to class file parsing.

### References

[Java Virtual Machine Specification](https://docs.oracle.com/javase/specs/jvms/se21/html/index.html)

[Java API Documentation](https://docs.oracle.com/javase/8/docs/api/overview-summary.html)
