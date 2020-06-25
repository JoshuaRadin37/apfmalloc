main: malloc_test.o liblrmalloc_rs_global.a
	gcc -o main malloc_test.o -L. -llrmalloc_rs_global

malloc_test.o: malloc_test.c
	gcc -c malloc_test.c

liblrmalloc_rs_global.a: ../../target/debug/liblrmalloc_rs_global.a
	cp ../../target/debug/liblrmalloc_rs_global.a liblrmalloc_rs_global.a

../../target/debug/liblrmalloc_rs_global.a: ../../lrmalloc-rs-global/src/lib.rs
	cargo build --manifest-path=../../Cargo.toml --workspace

clean:
	rm main *.o liblrmalloc_rs_global.a
