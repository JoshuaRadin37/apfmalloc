expr: functions.o main.o parse_tree.o recursive_parser.o stack.o table_parser.o liblrmalloc_rs_global.a
	gcc -g -o expr functions.o main.o parse_tree.o recursive_parser.o stack.o table_parser.o -L. -llrmalloc_rs_global -lm

main.o: main.c parse_tree.o recursive_parser.o table_parser.o
	gcc -g -c -std=c99 -Wall -Werror -Wpedantic main.c

functions.o: functions.c functions.h
	gcc -g -c -std=c99 -Wall -Werror -Wpedantic functions.c

parse_tree.o: parse_tree.c parse_tree.h functions.o
	gcc -g -c -std=c99 -Wall -Werror -Wpedantic parse_tree.c

stack.o: stack.c stack.h
	gcc -g -c -std=c99 -Wall -Werror -Wpedantic stack.c

table_parser.o: table_parser.c table_parser.h parse_tree.o
	gcc -g -c -std=c99 -Wall -Werror -Wpedantic table_parser.c

recursive_parser.o: recursive_parser.c recursive_parser.h parse_tree.o
	gcc -g -c -std=c99 -Wall -Werror -Wpedantic recursive_parser.c

liblrmalloc_rs_global.a: ../../target/debug/liblrmalloc_rs_global.a
	cp ../../target/debug/liblrmalloc_rs_global.a liblrmalloc_rs_global.a

../../target/debug/liblrmalloc_rs_global.a: ../../*/*.rs ../../lrmalloc-rs-global/src/lib.rs
	cargo build --manifest-path=../../Cargo.toml --workspace

clean:
	rm functions.o main.o parse_tree.o stack.o table_parser.o recursive_parser.o liblrmalloc_rs_global.a
