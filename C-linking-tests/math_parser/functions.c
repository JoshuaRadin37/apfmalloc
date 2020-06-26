//
// Created by jradi on 2/20/2018.
//

#include "functions.h"
#include <string.h>
#include <math.h>

#define PI (4.0*atan(1.0)) //Calculating pi live



//gets the number of parameters for a function, return -1 if the function doesn't exist
int num_Of_Params_For_Function(char* func){
	if(strcmp(func,"sin") == 0){
		return 1;
	}
	if(strcmp(func,"cos") == 0){
		return 1;
	}
	if(strcmp(func,"tan") == 0){
		return 1;
	}
	if(strcmp(func,"sind") == 0){
		return 1;
	}
	if(strcmp(func,"cosd") == 0){
		return 1;
	}
	if(strcmp(func,"tand") == 0){
		return 1;
	}

	if(strcmp(func,"arcsin") == 0){
		return 1;
	}
	if(strcmp(func,"arccos") == 0){
		return 1;
	}
	if(strcmp(func,"arctan") == 0){
		return 1;
	}
	if(strcmp(func,"arcsind") == 0){
		return 1;
	}
	if(strcmp(func,"arccosd") == 0){
		return 1;
	}
	if(strcmp(func,"arctand") == 0){
		return 1;
	}


	if(strcmp(func,"sqrt") == 0){
		return 1;
	}
	if(strcmp(func,"pow") == 0){
		return 2;
	}

	if(strcmp(func,"pi") == 0){
		return 1;
	}
	if(strcmp(func,"vlength") == 0){
		return 4;
	}

	return -1;
}

//not used because the num rules can achieve the same thing
char funcExists(char* func){
	char *funcs[] = {"sin", "cos", "tan", "sind", "cosd", "tand","arcsin","arccos","arctan","arccosd","arcsind","arctand", "sqrt", "pow","pi", "vlength", 0};
	char *ptr = funcs[0];
	int i = 0;
	while(ptr != NULL){
		if(strcmp(func,ptr) == 0) return 1;
		ptr = funcs[++i];
	}

	return 0;
}


double func_sin(double x){
	return sin(x);
}

double func_cos(double x){
	return cos(x);
}

double func_tan(double x){
	return tan(x);
}

double func_sqrt(double x){
	return sqrt(x);
}

double func_pow(double x, double r){
	//if(r == 0) return 1;
	return pow(x, r);
}

double func_sin_deg(double x){
	return sin(x*PI/180);
}
double func_cos_deg(double x){
	return cos(x*PI/180);
}
double func_tan_deg(double x){
	return tan(x*PI/180);
}

double func_inv_sin(double x){
	return asin(x);
}
double func_inv_cos(double x){
	return acos(x);
}
double func_inv_tan(double x){
	return atan(x);
}

double func_inv_sind(double x){
	return asin(x)*180/PI;
}
double func_inv_cosd(double x){
	return acos(x)*180/PI;
}
double func_inv_tand(double x){
	return atan(x)*180/PI;
}

//returns pi*x
double func_pi(double x){
	return PI*x;
}

//returns the length of a vector from point (x1,y1) to (x2,y2)
double func_vlength(double x1, double y1, double x2, double y2){
	return sqrt(pow(x2-x1,2)+pow(y2-y1,2));
}
