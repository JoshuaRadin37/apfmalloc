#include <stdio.h>
#include "apfmalloc.h"

typedef struct tree_node {
	void* value;
	struct tree_node* left;
	struct tree_node* right;
} tree_node;

tree_node* new_node(void* data) {
	tree_node* output = malloc(sizeof(tree_node));
	output->value = data;
	output->left = NULL;
	output->right = NULL;
	return output;
}

void free_tree(tree_node* node) {
	free(node->value);
	if (node->left) free_tree(node->left);
	if (node->right) free_tree(node->right);
	free(node);
}


int main(int argc, char* argv[]) {
    int* test = malloc(sizeof(int));
    *test = 3;
    // free(test);
    
    tree_node* node1 = new_node(NULL);
    node1->value = test;
    
    free_tree(node1);
    
    unsigned char b = check_override();
    printf("Check Override output: %d\n", b);
    
	return !b;
}
