//
// Created by jradi on 2/20/2018.
//

#ifndef SRC_FUNCTIONS_H
#define SRC_FUNCTIONS_H


int num_Of_Params_For_Function(char* func);

double func_sin(double x);
double func_cos(double x);
double func_tan(double x);

double func_sin_deg(double x);
double func_cos_deg(double x);
double func_tan_deg(double x);

double func_inv_sin(double x);
double func_inv_cos(double x);
double func_inv_tan(double x);

double func_inv_sind(double x);
double func_inv_cosd(double x);
double func_inv_tand(double x);

double func_sqrt(double x);

double func_pow(double x, double r);

double func_pi(double x);

double func_vlength(double x1, double y1, double x2, double y2);

char funcExists(char* func);
#endif //SRC_FUNCTIONS_H
