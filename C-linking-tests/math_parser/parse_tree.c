//
// Created by jradi on 2/14/2018.
//

#include <stdlib.h>
#include <stdio.h>
#include <string.h>
#include <math.h>
#include "parse_tree.h"
#include "functions.h"

#define SPACER "   "

Parse_Node_Extended empty = {0};


int is_Symbol_Digit(char c){
	return ((int) c >= (int) '0' && (int) c <= (int) '9') ? 1 : 0;
}

int is_Symbol_Char(char c){
	return ((int) c >= (int) 'a' && (int) c <= 'z') ? 1 : 0;
}

//finds the first node with the given category. This node must have no children, and the more left a node is the
//higher the priority has. Used for the table parser.
Parse_Node_Extended* find_Parent(Parse_Node_Extended* root, char* category){
	if(root == NULL) return NULL;
	if(root->child_count == 0 && strcmp(root->category,category) == 0) return root;
	for(int i = 0; i < root->child_count; i++){
		Parse_Node_Extended* output = find_Parent(root->children[i], category);
		if (output != NULL) return output;
	}
	return NULL;
}


Parse_Node_Extended* create_Extended_Node(char* category){
	Parse_Node_Extended* output = (Parse_Node_Extended*)(malloc(sizeof(Parse_Node_Extended) + sizeof(Parse_Node_Extended*)*2));
	output->category = category;
	output->child_count = 0;
	output->children = (Parse_Node_Extended**)(malloc(sizeof(Parse_Node_Extended*)));
	output->children[0] = NULL;
	return output;
}

/// Adds a child to an extended Node
/// \param node the node being added to
/// \param child the new child for the node
void add_Child_To_Extended_Node(Parse_Node_Extended* node, Parse_Node_Extended* child){

	node->child_count++;
	node->children = (Parse_Node_Extended**)(realloc(node->children, (node->child_count+1)*(sizeof(Parse_Node_Extended*))));

	if(node->children == NULL) return;
	node->children[node->child_count-1] = child;
	node->children[node->child_count] = NULL;
}


Parse_Node* create_Parse_Node_From_Extended(Parse_Node_Extended* ex){
	Parse_Node* output = create_Parse_Node();
	output->category = ex->category;
	return output;
}

//creates the left child right sibling tree nodes
Parse_Node* create_Tree_At_Point(Parse_Node_Extended* point, Parse_Node_Extended* parent, int child_Index){
	Parse_Node* output = create_Parse_Node_From_Extended(point);


	if(parent->child_count > 0){
		if(child_Index < parent->child_count-1){
			output->right_Sibling = create_Tree_At_Point(parent->children[child_Index+1], parent, child_Index+1);
		}
	}

	if(point->child_count > 0) output->left_most_child = create_Tree_At_Point(point->children[0], point, 0);

	return output;
}

//makes the process of reading and printing the parse tree a lot easier
Parse_Tree* create_Parse_Tree_From_Extended_Nodes(Parse_Node_Extended* root){
	Parse_Tree* output = create_Tree_At_Point(root, &empty, 0);
	free_Extended_Tree(root);
	return output;
}

//frees every single parse node in the tree
void free_Tree(Parse_Node* tree){
	if(tree->right_Sibling != NULL) free_Tree(tree->right_Sibling);
	if(tree->left_most_child != NULL) free_Tree(tree->left_most_child);

	free(tree);
}


//frees every extended node in the extended parse tree
void free_Extended_Tree(Parse_Node_Extended* tree){
	for(int i = 0; i < tree->child_count; i++){
		free_Extended_Tree(tree->children[i]);
	}

	free(tree->children);
	free(tree);
}


//during table parsing, the parser allocs memory for the digits. These need to be removed.
void flush_Chars_and_Digits(Parse_Node_Extended* tree){

	for(int i = 0; i < tree->child_count; i++){

		if(tree->children[i]->category[0] == '\\' && tree->children[i]->category[2] == '$'){
			printf("\nFreeing %s", tree->children[i]->category);
			free(tree->children[i]->category);
		}


		else flush_Chars_and_Digits(tree->children[i]);
	}
}

//removes any part of the tree that that points to empty ("////")
void trim_Parse_Tree(Parse_Tree* tree){
	if(tree->right_Sibling != NULL &&
	   tree->right_Sibling->left_most_child != NULL &&
	   strcmp(tree->right_Sibling->left_most_child->category,"\\\\") == 0){

		//printf("DELETING %s (%s)\n", tree->right_Sibling->category, tree->right_Sibling->left_most_child->category);
		Parse_Tree* new_right = tree->right_Sibling->right_Sibling;
		free(tree->right_Sibling->left_most_child);
		free(tree->right_Sibling);
		tree->right_Sibling = new_right;
	}
	if(tree->right_Sibling != NULL) trim_Parse_Tree(tree->right_Sibling);
	if(tree->left_most_child != NULL) trim_Parse_Tree(tree->left_most_child);
}

//empties allocs from the table, noted by the appended '$' character
void free_Table_Extra_Allocs(Parse_Tree* tree){
	if(tree == NULL) return;
	if(tree->category[2] == '$'){
		printf("Attempting to free %s\n", tree->category);
		free(tree->category);
	}
	free_Table_Extra_Allocs(tree->left_most_child);
	free_Table_Extra_Allocs(tree->right_Sibling);
}


//Deprecated
Parse_Tree* create_Parse_Tree(char* category){
	Parse_Tree* output = (Parse_Tree*)(malloc(sizeof(struct Parse_Node)*2));
	output->category = category;
	output->right_Sibling = NULL;
	output->left_most_child = NULL;

	return output;
}

//initalizes the parse node
Parse_Node* create_Parse_Node(){
	Parse_Tree* output = (Parse_Tree*)(malloc(sizeof(struct Parse_Node)*2));
	output->left_most_child = output->right_Sibling = NULL;
	return output;
}

void print_Parse_Tree(Parse_Tree* head){
	if(head == NULL) return;
	print_Parse_Tree_With_Indent(head, 0);
}

/*
 * prints it like this:
 *  root
 *      root.left_child
 *          root.left_child.left_child
 *              ...
 *          root.left_child.left_child.right_sibling
 *      root.left_child.right_sibling
 *          ...
 */
void print_Parse_Tree_With_Indent(Parse_Tree* head, int indent){

	for(int i = 0; i < indent; i++){
		printf(SPACER);
	}

	if(head->category[0] != '\\') printf("%s\n", head->category);
	else printf("%c\n", head->category[1]);


	if(head->left_most_child != NULL){
		print_Parse_Tree_With_Indent(head->left_most_child, indent+1);
	}
	if(head->right_Sibling != NULL){
		print_Parse_Tree_With_Indent(head->right_Sibling, indent);
	}
}

void print_Prefix_Transversal(Parse_Tree* head){
	if(head == NULL) return;
	if(head->category[0] == '\\') printf("%c", head->category[1]);
	print_Prefix_Transversal(head->left_most_child);
	print_Prefix_Transversal(head->right_Sibling);

}


//converts a char to to its double value
double get_val_From_Char(char c){
	double value = (double)(c - (int) '0'); //changed from flat 48 value to whatever the int value of char '0' is. Assumes that 0-9 are in a row in the computer
	if(value < 0 || value > 9) return -1;
	return value;
}

//Allocates a string of chars based of the <string> syntactic category
char* value_Of_String_Parse_Node(Parse_Node* node){
	int size = 1;
	Parse_Node* ptr = node->left_most_child->right_Sibling;
	while(ptr != NULL){
		size++;
		ptr = ptr->left_most_child->left_most_child->right_Sibling;
	}

	char* output = (char*)(malloc(sizeof(char)*(size+1)));
	ptr = node;
	for (int i = 0; i < size; ++i) {
		output[i] = ptr->left_most_child->left_most_child->category[1];
		free(ptr->left_most_child->left_most_child->category); //this frees the chars that were created in the parsing process
		if(i<size-1)ptr = ptr->left_most_child->right_Sibling->left_most_child;
	}

	output[size] = '\0';


	return output;
}


//recursive solution to determining the value of an expression.
//returns values as a pointer to a double so that I can return NULL to represent either not a number, division by 0, or a misfed function
double* value_Of_Parse_Node(Parse_Node* node){
	double* output = (double*)(malloc(sizeof(double)));

	char* category = node->category;

	Parse_Node *left, *center;

	left = node->left_most_child;
	if(left == NULL) return NULL;
	center = left->right_Sibling;


	double val1,val2;
	double *locVal1, *locVal2;


	if(strcmp(category,"<expression>") == 0){
		locVal1 = value_Of_Parse_Node(left);
		if(locVal1 == NULL) return NULL;
		val1 = *locVal1;
		free(locVal1);
		Parse_Node* ptr = center;
		while(ptr != NULL) {
			//because of the structure of the grammar, the grammar couldn't create a parse tree that was
			//unambiguous, left-recursive, and in the order of operations.
			//This while loop makes up for this by changing the direction it calculates multiple similar + and -
			//in a row from reading right to left, as the tree is created, to reading it from left to right in the order of OOP
			//EX: before: 5-4+1 = 0 (False), after: 5-4+1 = 2 (True)
			char operator = ptr->left_most_child->category[1];
			locVal2 =  value_Of_Parse_Node(ptr->left_most_child->right_Sibling->left_most_child);

			if(ptr->left_most_child->category[2] == '$') free(ptr->left_most_child->category);

			if(locVal2 == NULL) return NULL;
			val2 = *locVal2;
			free(locVal2);
			if(operator == '+') val1 += val2;
			else if(operator == '-') val1 -= val2;
			ptr = ptr->left_most_child->right_Sibling->left_most_child->right_Sibling;
		}

		output[0] = val1;

	}
	if(strcmp(category,"<group>") == 0){
		locVal1 = value_Of_Parse_Node(left);
		if(locVal1 == NULL) return NULL;
		val1 = *locVal1;
		free(locVal1);
		Parse_Node* ptr = center;
		while(ptr != NULL) {
			//because of the structure of the grammar, the grammar couldn't create a parse tree that was
			//unambiguous, left-recursive, and in the order of operations.
			//This while loop makes up for this by changing the direction it calculates multiple similar * and /
			//in a row from reading right to left, as the tree is created, to reading it from left to right in the order of OOP
			//EX: before: 5/3/2 = 3.33333 (False), after: 5/3/2 = 0.83333 (True)
			char operator = ptr->left_most_child->category[1];
			locVal2 =  value_Of_Parse_Node(ptr->left_most_child->right_Sibling->left_most_child);

			if(ptr->left_most_child->category[2] == '$') free(ptr->left_most_child->category);


			if(locVal2 == NULL) return NULL;
			val2 = *locVal2;
			free(locVal2);
			if(operator == '*') val1 *= val2;
			else if(operator == '/'){
				if(val2 == 0){
					printf("DIVIDE BY ZERO ERROR\n");
					return NULL;
				}
				val1 /= val2;
			}
			ptr = ptr->left_most_child->right_Sibling->left_most_child->right_Sibling;
		}





		output[0] = val1;

	}
	if(strcmp(category,"<factor>") == 0){

		if(left->category[1] == '-'){
			locVal1 = value_Of_Parse_Node(center);
			if(locVal1 == NULL) return NULL;
			val1 = -1**locVal1;
		}else if(left->category[1] == '('){
			locVal1 = value_Of_Parse_Node(center);
			if(locVal1 == NULL) return NULL;
			val1 = *locVal1;
		}else{
			locVal1 = value_Of_Parse_Node(left);
			if(locVal1 == NULL) return NULL;
			val1 = *locVal1;
		}

		free(locVal1);

		output[0] = val1;
	}
	if(strcmp(category,"<number>") == 0){
		locVal1 = value_Of_Parse_Node(left);
		val1 = *locVal1;
		free(locVal1);
		Parse_Node* ptr = center;
		while(ptr != NULL){
			locVal2 = value_Of_Parse_Node(ptr->left_most_child->left_most_child);
			val1 = val1*10+*locVal2;
			free(locVal2);
			ptr = ptr->left_most_child->left_most_child->right_Sibling;
		}

		output[0] = val1;
	}
	if(strcmp(category,"<digit>") == 0){
		output[0] = get_val_From_Char(left->category[1]);
		free(left->category);
	}
	if(strcmp(category,"<function>") == 0){
		char* func = value_Of_String_Parse_Node(left);
		double* values = get_Values_From_Param_List(center);
		if(values == NULL) return NULL;
		if(num_Of_Params_For_Function(func) != num_Of_Paramaters(center)){
			if(num_Of_Params_For_Function(func) == -1){
				printf("FUNC ERROR\t'%s' FUNCTION DOES NOT EXIST",func);
				return NULL;
			}
			printf("ARGUMENT ERROR\t'%s' params needed: %i\tparams found: %i\n",func,num_Of_Params_For_Function(func),num_Of_Paramaters(center));
			return NULL;
		}
		double* func_val=  value_From_Function(func, values);
		if(func_val == NULL){
			free(func_val);
			return NULL;
		}
		output[0] = *func_val;
		free(func_val);

		char* left_paren, *right_paren;
		left_paren = node->left_most_child->right_Sibling->left_most_child->category;
		if(node->left_most_child->right_Sibling->left_most_child->right_Sibling->right_Sibling->category[0] == '\\'){
			right_paren = node->left_most_child->right_Sibling->left_most_child->right_Sibling->right_Sibling->category;
		}
		else right_paren = node->left_most_child->right_Sibling->left_most_child->right_Sibling->right_Sibling->right_Sibling->category;

		if(left_paren[2] == '$'){
			free(left_paren);
			free(right_paren);
		}


		free(func);
		free(values);
	}

	return output;
}

//makes sure that head is not null before sending it to the evaluator
double* calculate_Value_From_Parse_Tree(Parse_Tree* head){
	if(head == NULL) return NULL;
	return value_Of_Parse_Node(head);
}



//calculates the number of paramater expressions that exist in a <paramlist>
int num_Of_Paramaters(Parse_Node *node){
	if(strcmp(node->category, "<paramlist>") != 0) return 0;


	if(node->left_most_child == NULL) return 0;
	int count = 1;
	Parse_Node *ptr = node->left_most_child->right_Sibling->right_Sibling;
	while(ptr != NULL && strcmp(ptr->category, "<ptail>") == 0){

		count++;
		ptr = ptr->left_most_child->right_Sibling->right_Sibling;
	}

	return count;
}

//returns a list of doubles from a param list
double* get_Values_From_Param_List(Parse_Node *node){
	int num_param = num_Of_Paramaters(node);
	double* list = (double*)(malloc(sizeof(double)*(num_param)));

	Parse_Node* ptr = node->left_most_child->right_Sibling;

	for(int i = 0; i < num_param; i++){
		Parse_Node* expression = ptr;


		double* locVal1 = value_Of_Parse_Node(expression);
		if(locVal1 == NULL) return NULL;
		double val1 = *locVal1;
		free(locVal1);
		list[i] = val1;if(i<num_param-1) {
			if (ptr->right_Sibling->left_most_child->category[2] == '$')
				free(ptr->right_Sibling->left_most_child->category);
			ptr = ptr->right_Sibling->left_most_child->right_Sibling;
		}

	}


	return list;
}

//takes in the name of the function and the list of doubles and returns the answer
double* value_From_Function(char* func, double *input){
	double output = 0;

	if(strcmp(func,"sin") == 0){
		output = func_sin(input[0]);
	}
	if(strcmp(func,"cos") == 0){
		output = func_cos(input[0]);
	}
	if(strcmp(func,"tan") == 0){
		output = func_tan(input[0]);
	}
	if(strcmp(func,"sind") == 0){
		output = func_sin_deg(input[0]);
	}
	if(strcmp(func,"cosd") == 0){
		output = func_cos_deg(input[0]);
	}
	if(strcmp(func,"tand") == 0){
		output = func_tan_deg(input[0]);
	}


	if(strcmp(func,"arcsin") == 0){
		output = func_inv_sin(input[0]);
	}
	if(strcmp(func,"arccos") == 0){
		output = func_inv_cos(input[0]);
	}
	if(strcmp(func,"arctan") == 0){
		output = func_inv_tan(input[0]);
	}
	if(strcmp(func,"arcsind") == 0){
		output = func_inv_sind(input[0]);
	}
	if(strcmp(func,"arccosd") == 0){
		output = func_inv_cosd(input[0]);
	}
	if(strcmp(func,"arctand") == 0){
		output = func_inv_tand(input[0]);
	}


	if(strcmp(func,"sqrt") == 0){
		if(input[0] < 0){
			printf("IMAGINARY ANSWER ERROR\n");
			return NULL;
		}
		output = func_sqrt(input[0]);
	}
	if(strcmp(func,"pow") == 0){
		if(input[0] < 0 && (input[1] > 0 && input[1] < 1)){
			printf("POTENTIAL IMAGINARY ANSWER ERROR\n");
			return NULL;
		}
		output = func_pow(input[0], input[1]);
	}

	if(strcmp(func,"pi") == 0){
		output = func_pi(input[0]);
	}
	if(strcmp(func,"vlength") == 0){
		output = func_vlength(input[0], input[1], input[2], input[3]);
	}


	double* output_ptr = (double*)(malloc(sizeof(double)));
	output_ptr[0] = output;
	return output_ptr;
}

