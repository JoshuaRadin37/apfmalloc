#include <stdio.h>
#include "parse_tree.h"
#include "recursive_parser.h"
#include "table_parser.h"
#include <string.h>
#include <stdlib.h>

void print_Shpeal();

int main(int argc, char *argv[]) {

	//Parse_Tree* tree = run_Table_Parser("sqrt(pow(cosd(30),2)+pow(sind(30),2))");


	int use_table = 0;

	if(argc == 1){
		print_Shpeal();
		return 0;
	}else if(argc == 2){
		if(argv[1][0] == 't') use_table = 1;
		else if(argv[1][0] == 'r') use_table = 0;
		else if(argv[1][0] == '?'){
			print_Shpeal();
		}
		else return 0;
	}else{
		return 0;
	}



	/* RESERVED WORDS
	 * -EXIT
	 * -exit
	*/


	char input[2000];

	if(use_table) printf("Running table parser\n");
	else printf("Running Recursive Descent Parser\n");

	while(1){
		printf("\n> ");
		scanf("%s", input);

		if(strcmp(input, "exit") == 0 || strcmp(input, "EXIT") == 0){
			break;
		}else{
			printf("Compiling %s\n", input);
			Parse_Tree* tree;
			if(use_table == 1){
				tree = run_Table_Parser(input);
			}else{
				tree = run_Recursive_Parser(input);
			}
			if(tree != NULL) {
				print_Parse_Tree(tree);
				printf("\n");
				print_Prefix_Transversal(tree);
				printf("\n");
				double* value = NULL;
				value = calculate_Value_From_Parse_Tree(tree);
				if(value != NULL) printf("\t= %g\n", *value);


				free_Tree(tree);
				free(value);
			}
		}

//		free(input);
	}



	return 0;
}

void print_Shpeal(){
	printf(
			"Expression Evaluator by Joshua Radin (2018) for CSC 173\n"
					"HOW TO USE\n"
					"\tCommand: ./expr [option]\n"
			"\nOptions:\n"
					"\t t = Run using Table Parser\n"
					"\t r = Run using Recursive Decsent Parser\n"
					"\t ? = Shows this information\n"
					"\nCalculator can be quit at anytime by typing 'EXIT' or 'exit'\n"
			"\nWhen inputing an expression, only whole numbers can be inputed.\n"
					"However, the expressions are evaluted using doubles, so all rational numbers can be expressed.\n"
			"\nFurthermore, functions can be used within expression.\n"
					"Functions are written as \"function_name(param1,param2...)\"\n"
					"Example: sqrt(pow(3,2)+pow(4,2))*cos(arctan(4/3))\n"
					"\tThis will return 3\n"
			"\nFUNCTIONS (x represents an expression):\n"
			"\tsin(x) - x in radians\n"
					"\tcos(x) - x in radians\n"
					"\ttan(x) - x in radians\n"
					"\tcosd(x) - x in degrees\n"
					"\tsind(x) - x in degrees\n"
					"\ttand(x) - x in degrees\n"
					"\tarcsin(x) - returns radians\n"
					"\tarccos(x) - returns radians\n"
					"\tarctan(x) - returns radians\n"
					"\tarcsind(x) - returns degrees\n"
					"\tarccosd(x) - returns degrees\n"
					"\tarctand(x) - returns degrees\n"
					"\tsqrt(x) - returns the square root of x if x >= 0\n"
					"\tpow(x, r) - returns x^r unless x < 0 and 0<r<1\n"
					"\tpi(x) - returns pi*x\n"
					"\tvlength(x1,y1,x2,y2) - returns the distance between points (x1,y1) and (x2,y2)\n"

	);
}

