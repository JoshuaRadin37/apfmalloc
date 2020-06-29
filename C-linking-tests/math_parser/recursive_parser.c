//
// Created by jradin on 2/26/2018.
//

#include "recursive_parser.h"
#include "parse_tree.h"
#include <stdlib.h>
#include <stdio.h>

char* look_ahead;
Parse_Node_Extended* head;

/*
 * General format:
 * All functions take in a parent extended node
 * Adds the result of the extended node to the parent at the end
 * Each functions moves the look_ahead if a terminal goal is completed
 */

int expression(Parse_Node_Extended* parent);
int etial(Parse_Node_Extended* parent);
int group(Parse_Node_Extended* parent);
int gtail(Parse_Node_Extended* parent);
int factor(Parse_Node_Extended* parent);
int number(Parse_Node_Extended* parent);
int ntail(Parse_Node_Extended* parent);
int digit(Parse_Node_Extended* parent);

int sc_char(Parse_Node_Extended* parent);
int string(Parse_Node_Extended* parent);
int stail(Parse_Node_Extended* parent);
int function(Parse_Node_Extended* parent);
int paramlist(Parse_Node_Extended* parent);
int ptail(Parse_Node_Extended* parent);

Parse_Tree* run_Recursive_Parser(char* input){

	look_ahead = input;
	Parse_Tree* output = NULL;
	if(expression(NULL) == 1){
		output = create_Parse_Tree_From_Extended_Nodes(head);
	}

	if(*look_ahead != '\0'){
		if(output != NULL) {
			free_Table_Extra_Allocs(output);
			free_Tree(output);
		}
		//free(output);
		printf("INVALID CHARACTER ERROR: %c\n", *look_ahead);
		return NULL;
	}

	return output;
}


int expression(Parse_Node_Extended* parent){
	Parse_Node_Extended* node_extended = create_Extended_Node("<expression>");
	if(*look_ahead == '\0') {flush_Chars_and_Digits(node_extended); free_Extended_Tree(node_extended); return 0;}
	
	if(group(node_extended) == 0) {flush_Chars_and_Digits(node_extended); free_Extended_Tree(node_extended); return 0;}
	if(etial(node_extended) == 0) {flush_Chars_and_Digits(node_extended); free_Extended_Tree(node_extended); return 0;}

	if(parent == NULL) head = node_extended;
	else add_Child_To_Extended_Node(parent, node_extended);
	return 1;
}
int etial(Parse_Node_Extended* parent){

	if(*look_ahead == '+' || *look_ahead == '-'){
		Parse_Node_Extended* node_extended = create_Extended_Node("<etail>");
		if(*look_ahead == '+') add_Child_To_Extended_Node(node_extended, create_Extended_Node("\\+"));
		else add_Child_To_Extended_Node(node_extended, create_Extended_Node("\\-"));


		look_ahead++;
		if(expression(node_extended) == 0) {flush_Chars_and_Digits(node_extended); free_Extended_Tree(node_extended); return 0;}
		add_Child_To_Extended_Node(parent, node_extended);
	}

	return 1;
}
int group(Parse_Node_Extended* parent){
	Parse_Node_Extended* node_extended = create_Extended_Node("<group>");
	if(*look_ahead == '\0') {flush_Chars_and_Digits(node_extended); free_Extended_Tree(node_extended); return 0;}

	if(factor(node_extended) == 0) {flush_Chars_and_Digits(node_extended); free_Extended_Tree(node_extended); return 0;}
	if(gtail(node_extended) == 0) {flush_Chars_and_Digits(node_extended); free_Extended_Tree(node_extended); return 0;}

	add_Child_To_Extended_Node(parent, node_extended);
	return 1;
}
int gtail(Parse_Node_Extended* parent){
	if(*look_ahead == '*' || *look_ahead == '/'){
		Parse_Node_Extended* node_extended = create_Extended_Node("<gtail>");
		if(*look_ahead == '*') add_Child_To_Extended_Node(node_extended , create_Extended_Node("\\*"));
		else add_Child_To_Extended_Node(node_extended , create_Extended_Node("\\/"));


		look_ahead++;
		if(group(node_extended) == 0) {flush_Chars_and_Digits(node_extended); free_Extended_Tree(node_extended); return 0;}
		add_Child_To_Extended_Node(parent, node_extended);
	}

	return 1;
}
int factor(Parse_Node_Extended* parent){
	Parse_Node_Extended* node_extended = create_Extended_Node("<factor>");
	if(*look_ahead == '\0') {flush_Chars_and_Digits(node_extended); free_Extended_Tree(node_extended); return 0;}

	if(is_Symbol_Digit(*look_ahead)){
		//look_ahead++;
		if(number(node_extended) == 0) {flush_Chars_and_Digits(node_extended); free_Extended_Tree(node_extended); return 0;}
	}else if(*look_ahead == '-'){
		add_Child_To_Extended_Node(node_extended, create_Extended_Node("\\-"));
		look_ahead++;
		if(factor(node_extended) == 0) {flush_Chars_and_Digits(node_extended); free_Extended_Tree(node_extended); return 0;}
	}else if(*look_ahead == '('){
		add_Child_To_Extended_Node(node_extended, create_Extended_Node("\\("));
		look_ahead++;
		if(expression(node_extended) == 0) {flush_Chars_and_Digits(node_extended); free_Extended_Tree(node_extended); return 0;}
		if(*look_ahead == ')'){
			add_Child_To_Extended_Node(node_extended, create_Extended_Node("\\)"));
			look_ahead++;
		}else {flush_Chars_and_Digits(node_extended); free_Extended_Tree(node_extended); return 0;}
	}else if(is_Symbol_Char(*look_ahead)){
		if(function(node_extended) == 0) {flush_Chars_and_Digits(node_extended); free_Extended_Tree(node_extended); return 0;}
	}



	add_Child_To_Extended_Node(parent, node_extended);
	return 1;
}
int number(Parse_Node_Extended* parent){
	Parse_Node_Extended* node_extended = create_Extended_Node("<number>");
	if(*look_ahead == '\0') {flush_Chars_and_Digits(node_extended); free_Extended_Tree(node_extended); return 0;}

	if(digit(node_extended) == 0) {flush_Chars_and_Digits(node_extended); free_Extended_Tree(node_extended); return 0;}
	if(ntail(node_extended) == 0) {flush_Chars_and_Digits(node_extended); free_Extended_Tree(node_extended); return 0;}

	add_Child_To_Extended_Node(parent, node_extended);
	return 1;
}
int ntail(Parse_Node_Extended* parent){
	if(is_Symbol_Digit(*look_ahead)) {
		Parse_Node_Extended *node_extended = create_Extended_Node("<ntail>");
		if(number(node_extended) == 0) {flush_Chars_and_Digits(node_extended); free_Extended_Tree(node_extended); return 0;}
		add_Child_To_Extended_Node(parent, node_extended);
	}

	return 1;
}
int digit(Parse_Node_Extended* parent){
	Parse_Node_Extended *node_extended = create_Extended_Node("<digit>");
	if(*look_ahead == '\0') {flush_Chars_and_Digits(node_extended); free_Extended_Tree(node_extended); return 0;}

	if(is_Symbol_Digit(*look_ahead) == 0) {flush_Chars_and_Digits(node_extended); free_Extended_Tree(node_extended); return 0;}
	char value = *look_ahead;
	look_ahead++;

	char* digit_string = (char*)(malloc(sizeof(char)*4));
	digit_string[0] = '\\';
	digit_string[1] = value;
	digit_string[2] = '$';
	digit_string[3] = '\0';

	Parse_Node_Extended* terminal = create_Extended_Node(digit_string);


	add_Child_To_Extended_Node(node_extended, terminal);
	add_Child_To_Extended_Node(parent, node_extended);
	return 1;
}

int sc_char(Parse_Node_Extended* parent){
	Parse_Node_Extended *node_extended = create_Extended_Node("<char>");
	if(*look_ahead == '\0') {flush_Chars_and_Digits(node_extended); free_Extended_Tree(node_extended); return 0;}

	if(is_Symbol_Char(*look_ahead) == 0) {flush_Chars_and_Digits(node_extended); free_Extended_Tree(node_extended); return 0;}
	char value = *look_ahead;
	look_ahead++;

	char* digit_string = (char*)(malloc(sizeof(char)*4));
	digit_string[0] = '\\';
	digit_string[1] = value;
	digit_string[2] = '$';
	digit_string[3] = '\0';
	Parse_Node_Extended* terminal = create_Extended_Node(digit_string);

	add_Child_To_Extended_Node(node_extended, terminal);
	add_Child_To_Extended_Node(parent, node_extended);
	return 1;
}
int string(Parse_Node_Extended* parent){
	Parse_Node_Extended* node_extended = create_Extended_Node("<string>");
	if(*look_ahead == '\0') {flush_Chars_and_Digits(node_extended); free_Extended_Tree(node_extended); return 0;}

	if(sc_char(node_extended) == 0) {flush_Chars_and_Digits(node_extended); free_Extended_Tree(node_extended); return 0;}
	if(stail(node_extended) == 0) {flush_Chars_and_Digits(node_extended); free_Extended_Tree(node_extended); return 0;}

	add_Child_To_Extended_Node(parent, node_extended);
	return 1;
}
int stail(Parse_Node_Extended* parent){
	if(is_Symbol_Char(*look_ahead)) {
		Parse_Node_Extended *node_extended = create_Extended_Node("<stail>");
		if(string(node_extended) == 0) {flush_Chars_and_Digits(node_extended); free_Extended_Tree(node_extended); return 0;}
		add_Child_To_Extended_Node(parent, node_extended);
	}

	return 1;
}
int function(Parse_Node_Extended* parent){
	Parse_Node_Extended* node_extended = create_Extended_Node("<function>");
	if(*look_ahead == '\0') {flush_Chars_and_Digits(node_extended); free_Extended_Tree(node_extended); return 0;}

	if(string(node_extended) == 0) {flush_Chars_and_Digits(node_extended); free_Extended_Tree(node_extended); return 0;}
	if(paramlist(node_extended) == 0) {flush_Chars_and_Digits(node_extended); free_Extended_Tree(node_extended); return 0;}

	add_Child_To_Extended_Node(parent, node_extended);
	return 1;
}
int paramlist(Parse_Node_Extended* parent){
	Parse_Node_Extended *node_extended = create_Extended_Node("<paramlist>");
	if(*look_ahead == '\0') {flush_Chars_and_Digits(node_extended); free_Extended_Tree(node_extended); return 0;}

	if(*look_ahead != '(') {flush_Chars_and_Digits(node_extended); free_Extended_Tree(node_extended); return 0;}
	look_ahead++;
	add_Child_To_Extended_Node(node_extended, create_Extended_Node("\\("));
	if(expression(node_extended) == 0) {flush_Chars_and_Digits(node_extended); free_Extended_Tree(node_extended); return 0;}
	if(ptail(node_extended) == 0) {flush_Chars_and_Digits(node_extended); free_Extended_Tree(node_extended); return 0;}
	if(*look_ahead != ')') {flush_Chars_and_Digits(node_extended); free_Extended_Tree(node_extended); return 0;}
	look_ahead++;
	add_Child_To_Extended_Node(node_extended, create_Extended_Node("\\)"));

	add_Child_To_Extended_Node(parent, node_extended);
	return 1;
}
int ptail(Parse_Node_Extended* parent){


	if(*look_ahead == ',') {
		look_ahead++;
		Parse_Node_Extended *node_extended = create_Extended_Node("<ptail>");
		add_Child_To_Extended_Node(node_extended, create_Extended_Node("\\,"));
		if(expression(node_extended) == 0) {flush_Chars_and_Digits(node_extended); free_Extended_Tree(node_extended); return 0;}
		if(ptail(node_extended) == 0) {flush_Chars_and_Digits(node_extended); free_Extended_Tree(node_extended); return 0;}

		add_Child_To_Extended_Node(parent, node_extended);
	}

	return 1;
}

