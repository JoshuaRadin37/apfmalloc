//
// Created by jradi on 2/24/2018.
//

#include "table_parser.h"
#include "parse_tree.h"
#include <stdlib.h>
#include <stdio.h>
#include "stack.h"
#include <string.h>


int single_Line_Parse(char* category, char lookahead);



/*
 * Symbols
 * \\c = terminal symbol c
 * $d = digit
 * $c = char
 */

/*
 * PRODUCTION FOR TABLE PARSER
 * $ represents empty
 *
 * RULE #| PRODUCTION
 * 0    <expression> -> <group><etail>
 *
 * 10   <etail> -> +<expression>
 * 11   <etail> -> $
 * 12   <etail> -> -<expression>
 *
 * 20   <group> -> <factor><gtail>
 *
 * 30   <gtail> -> *<group>
 * 31   <gtail> -> $
 * 32   <gtail> -> /<group>
 *
 * 40   <factor> -> <number>
 * 41   <factor> -> -<factor>
 * 42   <factor> -> (<expression>)
 * 43   <factor> -> <function>
 *
 * 50   <number> -> <digit><ntail>
 *
 * 60   <ntail> -> <number>
 * 61   <ntail> -> $
 *
 * 70   <digit> -> [0-9]
 *
 * 80   <string> -> <char><stail>
 *
 * 90   <stail> -> <string>
 * 91   <stail> -> $
 *
 * 100  <char> -> [a-z]
 *
 * 110  <function> -> <string><paramlist>
 *
 * 120  <paramlist> -> (<expression><ptail>)
 *
 * 130  <ptail> -> ,<expression><ptail>
 * 131  <ptail> -> $
 *
 * Yes the etail and gtail have a weird order. Thats because I forgot to split them into two rules after I created the
 * spreadsheet for the parse table
 */


int size_of_List_SC(char** list){
	int count = 0;

	int index = 0;
	while(list[index] != NULL){
		count++;
		index++;
	}
	return count;
}

Parse_Tree* run_Table_Parser(char* input){
	Stack* stack = create_Stack();
	push_To_Stack_Cat("<expression>", stack);
	Parse_Tree* tree = NULL;

	Parse_Node_Extended* root = create_Extended_Node("<expression>");

	char look_ahead = input[0];
	int index = 0;

	int terminals_created = 0;


	char* current;
	char** single_line_output;

	while(!stack_Empty(stack)){
		//printf("LOOK_AHEAD: %c\tSTACK: ", look_ahead);
		print_Stack(stack);
		current = pop_From_Stack(stack);

		Parse_Node_Extended* parent = find_Parent(root, current);

		int found = 0;
		int was_epsilon = 0;

		if(current[0] == '\\'){
			if(current[1] == look_ahead) found = 1;
			if(current[1] == '\\'){
				found = 1;
				was_epsilon = 1;
			}
		}else if(current[0] == '$'){
			if(current[1] == 'd' && is_Symbol_Digit(look_ahead)) found = 1;
			if(current[1] == 'c' && is_Symbol_Char(look_ahead)) found = 1;
		}


		if(found == 1){
			//pop_From_Stack(stack);
			if(was_epsilon == 0){

					char *digit = (char *) (malloc(sizeof(char) * 4));
					digit[0] = '\\';
					digit[1] = look_ahead;
					digit[2] = '$';
					digit[3] = '\0';

					//printf("Created %s at 0x%p\n", digit, (void *) digit);

					parent->category = digit;
					look_ahead = input[++index];

					terminals_created++;

			}
		}else{
			int rule = single_Line_Parse(current, look_ahead);
			if(rule == -1){
				free_Stack(stack);
				free(stack);
				flush_Chars_and_Digits(root);
				free_Extended_Tree(root);
				return NULL;
			}
			//printf("  %s %i", " - FOUND RULE:", rule);
			single_line_output = rule_Num_To_List_Of_Strings(rule);

			int size = size_of_List_SC(single_line_output);
			for(int i = size-1; i>=0; i--){
				Parse_Node_Extended* next = create_Extended_Node(single_line_output[size-1-i]);
				add_Child_To_Extended_Node(parent, next);
				push_To_Stack_Cat(single_line_output[i], stack);
			}
			free(single_line_output);
		}

		printf("\n");

	}

	if(look_ahead != '\0'){ //The entire setence wasn't parsed, so it wasn't in the language
		printf("INVALID CHARACTER ERROR\n");
		free_Stack(stack);
		free(stack);
		flush_Chars_and_Digits(root);
		free_Extended_Tree(root);
		return NULL;
	}

	tree = create_Parse_Tree_From_Extended_Nodes(root);
	trim_Parse_Tree(tree);
	//flush_Chars_and_Digits(root);
	//free_Extended_Tree(root);
	free(stack);


	//printf("SPACE USED FOR TERMINALS: %lu bits (size of char*: %lu bits)\n", (unsigned long) sizeof(char*)*3*terminals_created, (unsigned long) sizeof(char*));

	return tree;
}


///
/// \param category category
/// \param lookahead the lookahead symbol
/// \return the rule number to use, -1 if none found
int single_Line_Parse(char* category, char lookahead){

	if(strcmp(category, "<char>") == 0) {
		if(is_Symbol_Char(lookahead)) return 100;
	}
	if(strcmp(category, "<string>") == 0) {
		return 80;
	}
	if(strcmp(category, "<stail>") == 0) {
		if(is_Symbol_Char(lookahead)) return 90;
		if(lookahead != '\0') return 91;
	}
	if(strcmp(category, "<stail>") == 0) {
		if(is_Symbol_Char(lookahead)) return 90;
		if(lookahead != '\0') return 91;
	}
	if(strcmp(category, "<paramlist>") == 0) {
		if(lookahead == '(') return 120;
	}
	if(strcmp(category, "<ptail>") == 0) {
		if(lookahead == ',') return 130;
		return 131;
	}
	if(strcmp(category, "<function>") == 0) {
		return 110;
	}

	if(strcmp(category, "<digit>") == 0) {
		if(is_Symbol_Digit(lookahead)) return 70;
	}
	if(strcmp(category, "<number>") == 0) {
		return 50;
	}
	if(strcmp(category, "<ntail>") == 0) {
		if(is_Symbol_Digit(lookahead)) return 60;
		return 61;
	}
	if(strcmp(category, "<factor>") == 0) {
		if(is_Symbol_Digit(lookahead)) return 40;
		if(is_Symbol_Char(lookahead)) return 43;
		if(lookahead == '(') return 42;
		if(lookahead == '-') return 41;
	}
	if(strcmp(category, "<group>") == 0) {
		return 20;
	}
	if(strcmp(category, "<gtail>") == 0) {
		if(lookahead == '*') return 30;
		if(lookahead == '/') return 32;
		return 31;
	}
	if(strcmp(category, "<expression>") == 0) {
		return 0;
	}
	if(strcmp(category, "<etail>") == 0) {
		if(lookahead == '+') return 10;
		if(lookahead == '-') return 12;
		return 11;
	}

	return -1;
}

char** quick_malloc(int size){
	return (char**)(calloc((size_t ) size+1, sizeof(char*)));
}

//NULL TERMINATED LIST
//Terminal symbols have \\ before them, empty represented by "\\\\"

char** rule_Num_To_List_Of_Strings(int rule){
	char** output = NULL;
	if(rule == 0){
		output = quick_malloc(2);
		output[0] = "<group>";
		output[1] = "<etail>";
	}

	if(rule == 10){
		output = quick_malloc(2);
		output[0] = "\\+";
		output[1] = "<expression>";
	}
	if(rule == 11){
		output = quick_malloc(1);
		output[0] = "\\\\";
	}
	if(rule == 12){
		output = quick_malloc(2);
		output[0] = "\\-";
		output[1] = "<expression>";
	}

	if(rule == 20){
		output = quick_malloc(2);
		output[0] = "<factor>";
		output[1] = "<gtail>";
	}

	if(rule == 30){
		output = quick_malloc(2);
		output[0] = "\\*";
		output[1] = "<group>";
	}
	if(rule == 31){
		output = quick_malloc(1);
		output[0] = "\\\\";
	}
	if(rule == 32){
		output = quick_malloc(2);
		output[0] = "\\/";
		output[1] = "<group>";
	}

	if(rule == 40){
		output = quick_malloc(1);
		output[0] = "<number>";
	}
	if(rule == 41){
		output = quick_malloc(2);
		output[0] = "\\-";
		output[1] = "<factor>";
	}
	if(rule == 42){
		output = quick_malloc(3);
		output[0] = "\\(";
		output[1] = "<expression>";
		output[2] = "\\)";
	}
	if(rule == 43){
		output = quick_malloc(1);
		output[0] = "<function>";
	}

	if(rule == 50){
		output = quick_malloc(2);
		output[0] = "<digit>";
		output[1] = "<ntail>";
	}

	if(rule == 60){
		output = quick_malloc(1);
		output[0] = "<number>";
	}
	if(rule == 61){
		output = quick_malloc(1);
		output[0] = "\\\\";
	}

	if(rule == 70){
		output = quick_malloc(1);
		output[0] = "$d";
	}

	if(rule == 80){
		output = quick_malloc(2);
		output[0] = "<char>";
		output[1] = "<stail>";
	}

	if(rule == 90){
		output = quick_malloc(1);
		output[0] = "<string>";
	}
	if(rule == 91){
		output = quick_malloc(1);
		output[0] = "\\\\";
	}

	if(rule == 100){
		output = quick_malloc(1);
		output[0] = "$c";
	}

	if(rule == 110){
		output = quick_malloc(2);
		output[0] = "<string>";
		output[1] = "<paramlist>";
	}

	if(rule == 120){
		output = quick_malloc(4);
		output[0] = "\\(";
		output[1] = "<expression>";
		output[2] = "<ptail>";
		output[3] = "\\)";
	}

	if(rule == 130){
		output = quick_malloc(3);
		output[0] = "\\,";
		output[1] = "<expression>";
		output[2] = "<ptail>";
	}
	if(rule == 131){
		output = quick_malloc(1);
		output[0] = "\\\\";
	}


	return output;
}

