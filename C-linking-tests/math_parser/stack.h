//
// Created by jradi on 2/24/2018.
//

#ifndef SRC_STACK_H
#define SRC_STACK_H

/*
 * Standard stack data structure
 */


typedef struct Stack_Node{
	struct Stack_Node *previous, *next;
	char* cat;
} Stack_Node;

typedef struct{
	Stack_Node* head;
	int size;
} Stack;

Stack* create_Stack();
void free_Stack(Stack *s);
void print_Stack(Stack* s);

void push_To_Stack_Cat(char* category, Stack* s);
void push_To_Stack_Char(char c, Stack* s);
char* peek_From_Stack(Stack* s);
char* pop_From_Stack(Stack* s);
char stack_Empty(Stack* s);

#endif //SRC_STACK_H
