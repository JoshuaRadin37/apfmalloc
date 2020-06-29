//
// Created by jradi on 2/14/2018.
//

#ifndef SRC_PARSE_TREE_H
#define SRC_PARSE_TREE_H



typedef struct Parse_Node {
	char* category;
	struct Parse_Node* left_most_child;
	struct Parse_Node* right_Sibling;
} Parse_Node;

typedef struct Parse_Node_Extended{
	char* category;
	unsigned int child_count;
	struct Parse_Node_Extended** children;
} Parse_Node_Extended;

Parse_Node_Extended empty;

typedef Parse_Node Parse_Tree;


void add_Child_To_Extended_Node(Parse_Node_Extended* node, Parse_Node_Extended* child);

void trim_Parse_Tree(Parse_Tree* tree);
void free_Tree(Parse_Node* tree);
void free_Extended_Tree(Parse_Node_Extended* tree);
void flush_Chars_and_Digits(Parse_Node_Extended* tree);
void free_Table_Extra_Allocs(Parse_Tree* tree);

//Creation functions
Parse_Tree* create_Parse_Tree_From_Extended_Nodes(Parse_Node_Extended* root);

Parse_Node* create_Parse_Node_From_Extended(Parse_Node_Extended* ex);
Parse_Node* create_Parse_Node();
Parse_Node* create_Tree_At_Point(Parse_Node_Extended* point, Parse_Node_Extended* parent, int child_Index);

Parse_Node_Extended* create_Extended_Node(char* category);
Parse_Node_Extended* find_Parent(Parse_Node_Extended* root, char* category);


//Print functions
void print_Parse_Tree(Parse_Tree* head);
void print_Parse_Tree_With_Indent(Parse_Tree* head, int indent);
void print_Prefix_Transversal(Parse_Tree* head);

//Calculation functions
double* calculate_Value_From_Parse_Tree(Parse_Tree* head);
int num_Of_Paramaters(Parse_Node *node);
double* get_Values_From_Param_List(Parse_Node *node);
double* value_From_Function(char* func, double *input);

int is_Symbol_Char(char c); //boolean
int is_Symbol_Digit(char c); //boolean


#endif //SRC_PARSE_TREE_H
