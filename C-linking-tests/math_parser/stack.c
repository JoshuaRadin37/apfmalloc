//
// Created by jradi on 2/24/2018.
//

#include "stack.h"
#include <stdlib.h>
#include <stdio.h>

Stack* create_Stack(){
	Stack* output = (Stack*)(malloc(sizeof(Stack)));
	output->head = NULL;
	output->size = 0;
	return output;
}

void free_Stack(Stack *s){
	Stack_Node* ptr = s->head;
	Stack_Node* ptr_next = NULL;
	//if(ptr != NULL) ptr_next = ptr->next;
	while(ptr != NULL){
		ptr_next = ptr->next;
		free(ptr);
		ptr = ptr_next;
	}

}




void print_Stack(Stack* s){
	Stack_Node *ptr = s->head;
	for(int i = 0; i < s->size; i++){
		if(ptr->cat[0] != '\\') printf("%s ", ptr->cat);
		else if(ptr->cat[1] != '\\') printf("%c ", ptr->cat[1]);
		ptr = ptr->next;
	}
}

Stack_Node* create_Node_Cat(char* category){
	Stack_Node* output = (Stack_Node*)(malloc(sizeof(Stack_Node)));
	output->cat = category;
	output->next = NULL;
	output->previous = NULL;
	return output;
}


void push_To_Stack_Cat(char* category, Stack* s){
	Stack_Node* new_Node = create_Node_Cat(category);
	Stack_Node* ptr = s->head;
	new_Node->next = ptr;
	if(ptr != NULL) ptr->previous = new_Node;
	s->head = new_Node;
	s->size++;
}



char* peek_From_Stack(Stack* s){
	return s->head->cat;
}

char* pop_From_Stack(Stack* s){
	Stack_Node *top, *rest;
	top = s->head;
	rest = top->next;
	if(rest != NULL) rest->previous = NULL;
	s->head=rest;
	char* outout = top->cat;
	free(top);
	s->size--;
	return outout;
}
char stack_Empty(Stack* s){
	return (char) (s->size==0 ? 1:0);
}

