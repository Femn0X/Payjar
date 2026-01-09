import ast as absytr
import sys
import re# Import re for the Lexer and Parser
class Token:
   """Represents a lexical token with its type and value."""
   def __init__(self, type, value):
       self.type = type
       self.value = value
   def __repr__(self):
       return f"Token({self.type}, {self.value})"
class Parser:
   """Parses a list of tokens into an Abstract Syntax Tree (AST)."""
   def __init__(self, tokens):
       self.tokens = tokens
       self.current_token = None
       self.token_index = -1
       self.advance()
   def advance(self):
       """Moves to the next token in the stream."""
       self.token_index += 1
       if self.token_index < len(self.tokens):self.current_token = self.tokens[self.token_index]
       else:self.current_token = None # End of tokens (EOF)
   def eat(self, token_type):
       """
       Consumes the current token if its type matches the expected type,
       then advances to the next token. Raises an error if types don't match.
       """
       if self.current_token and self.current_token.type == token_type:self.advance()
       else:raise Exception(f"Syntax Error: Expected {token_type}, but got {self.current_token.type if self.current_token else 'EOF'}. Current token index: {self.token_index}")
   def parse_main(self):
       """Parses the main program structure: public class Main(@self){...}"""
       self.eat('PUBLIC')
       self.eat('CLASS')
       class_name = self.current_token.value
       self.eat('MAIN') # Expecting 'main' as an IDENTIFIER
       self.eat('LPAREN')
       self.eat('AT')
       self.eat('SELF')
       self.eat('RPAREN')
       self.eat('LBRACE')
       main_body = self.parse_main_body()
       self.eat('RBRACE')
       return {"type": "main_definition", "name": class_name, "body": main_body}
   def parse_main_body(self):
       """Parses the statements within the main class body."""
       statements = []
       while self.current_token and self.current_token.type != 'RBRACE':
           if self.current_token.type == 'PRINT':statements.append(self.parse_print_statement())
           elif self.current_token.type == 'FOR':statements.append(self.parse_for_statement())
           elif self.current_token.type in ('VAR','CONST', 'LET'):statements.append(self.parse_variable_declaration())
           elif self.current_token.type == 'DEF':statements.append(self.parse_function_definition())
           elif self.current_token.type == 'PUBLIC':statements.append(self.parse_public_class_definition())
           elif self.current_token.type == 'CLASS':statements.append(self.parse_class_definition())
           elif self.current_token.type in ('IDENTIFIER','MAIN'):
               peek_token_idx = self.token_index + 1
               if peek_token_idx < len(self.tokens) and self.tokens[peek_token_idx].type == 'EQUAL':statements.append(self.parse_assignment_statement())
               elif peek_token_idx < len(self.tokens) and self.tokens[peek_token_idx].type == 'DOT':
                   # Member access or assignment (e.g., obj.field = 10; or obj.method();)
                   obj_name = self.current_token.value
                   self.eat('IDENTIFIER') # Consume the initial object identifier
                   obj_expr = {"type": "variable_access", "name": obj_name}
                   expr = self.parse_member_access(obj_expr) # This might consume the semicolon if it's an assignment
                   statements.append(expr)
                   # If it's a method call (not an assignment), it needs a semicolon
                   if expr["type"] == "member_access" and expr["is_call"]:self.eat('SEMICOLON')
               elif peek_token_idx < len(self.tokens) and self.tokens[peek_token_idx].type == 'LPAREN':
                   # Global function call as a statement (e.g., myFunction(arg);)
                   expr = self.parse_function_call_expression()
                   statements.append(expr)
                   self.eat('SEMICOLON')
               else:raise Exception(f"Syntax Error: Unexpected token in main body (expected assignment or call): {self.current_token.type if self.current_token else 'EOF'}")
           else:raise Exception(f"Syntax Error: Unexpected token in main body: {self.current_token.type if self.current_token else 'EOF'}")
       return statements
   def parse_public_class_definition(self):
    self.eat('PUBLIC')
    info=self.parse_class_definition()
    return info
   def parse_class_definition(self):
       """Parses a class definition."""
       self.eat('CLASS')
       class_name = self.current_token.value
       self.eat('IDENTIFIER')
       self.eat('LPAREN')
       self.eat('AT')
       self.eat('INNERSELF')
       self.eat('RPAREN')
       self.eat('LBRACE')
       members = []
       methods = []
       constructor = None
       while self.current_token and self.current_token.type != 'RBRACE':
           if self.current_token.type in ('CONST', 'LET'):members.append(self.parse_class_field_declaration())
           elif self.current_token.type == 'DEF':
               method_def = self.parse_function_definition(is_method=True)
               if method_def["name"] == "init":constructor = method_def
               else:methods.append(method_def)
           else:raise Exception(f"Syntax Error: Unexpected token in class body: {self.current_token.type if self.current_token else 'EOF'}")
       self.eat('RBRACE')
       return {"type": "class_definition", "name": class_name, "fields": members, "methods": methods, "constructor": constructor}
   def parse_class_field_declaration(self):
       """Parses a field declaration within a class (e.g., let myField = 10;)."""
       declaration_type = self.current_token.type
       self.advance() # Consume CONST or LET
       field_name = self.current_token.value
       self.eat('IDENTIFIER')
       initial_value = None
       if self.current_token and self.current_token.type == 'EQUAL':self.eat('EQUAL');initial_value = self.parse_expression()
       self.eat('SEMICOLON')
       return {"type": "field_declaration", "kind": declaration_type, "name": field_name, "value": initial_value}
   def parse_function_definition(self, is_method=False):
       """Parses a function or method definition."""
       self.eat('DEF')
       function_name = self.current_token.value
       if self.current_token.type == 'IDENTIFIER':
           self.eat('IDENTIFIER')
       elif self.current_token.type == 'MAIN':
           self.eat('MAIN')
       else:
           raise Exception(f"Syntax Error: Expected function name, but got {self.current_token.type if self.current_token else 'EOF'}")
       self.eat('LPAREN')
       parameters = []
       if self.current_token and self.current_token.type != 'RPAREN':
           if is_method and self.current_token.type == 'SELF':parameters.append(self.current_token.value);self.eat('SELF')
           elif not is_method and self.current_token.type == 'SELF':raise Exception("Syntax Error: The 'self' parameter is only allowed in class method definitions.")
           else:parameters.append(self.current_token.value);self.eat('IDENTIFIER')
           while self.current_token and self.current_token.type == 'COMMA':
               self.eat('COMMA')
               parameters.append(self.current_token.value)
               self.eat('IDENTIFIER')
       self.eat('RPAREN')
       self.eat('LBRACE')
       body = []
       while self.current_token and self.current_token.type != 'RBRACE':
           if self.current_token.type == 'PRINT':body.append(self.parse_print_statement())
           elif self.current_token.type == 'FOR':body.append(self.parse_for_statement())
           elif self.current_token.type in ('CONST', 'LET'):body.append(self.parse_variable_declaration())
           elif self.current_token.type in ('IDENTIFIER','MAIN'):
               peek_token_idx = self.token_index + 1
               if peek_token_idx < len(self.tokens) and self.tokens[peek_token_idx].type == 'EQUAL':body.append(self.parse_assignment_statement())
               elif peek_token_idx < len(self.tokens) and self.tokens[peek_token_idx].type == 'DOT':
                   # Member access or assignment (e.g., obj.field = 10; or obj.method();)
                   obj_name = self.current_token.value
                   self.eat('IDENTIFIER') # Consume the initial object identifier
                   obj_expr = {"type": "variable_access", "name": obj_name}
                   expr = self.parse_member_access(obj_expr) # This might consume the semicolon if it's an assignment
                   body.append(expr)
                   # If it's a method call (not an assignment), it needs a semicolon
                   if expr["type"] == "member_access" and expr["is_call"]:self.eat('SEMICOLON')
               elif peek_token_idx < len(self.tokens) and self.tokens[peek_token_idx].type == 'LPAREN':
                   # Global function call as a statement (e.g., myFunction(arg);)
                   expr = self.parse_function_call_expression()
                   body.append(expr)
                   self.eat('SEMICOLON')
               else:raise Exception(f"Syntax Error: Unexpected token in function body (expected assignment or call): {self.current_token.type if self.current_token else 'EOF'}")
           elif self.current_token.type == 'RETURN':body.append(self.parse_return_statement())
           elif self.current_token.type == 'FOR':body.append(self.parse_for_statement())
           else:raise Exception(f"Syntax Error: Unexpected token in function body: {self.current_token.type if self.current_token else 'EOF'}")
       self.eat('RBRACE')
       return {"type": "function_definition", "name": function_name, "parameters": parameters, "body": body, "is_method": is_method}
   def parse_return_statement(self):
       """Parses a return statement."""
       self.eat('RETURN')
       expression = self.parse_expression()
       self.eat('SEMICOLON')
       return {"type": "return_statement", "expression": expression}
   def parse_print_statement(self):
       """Parses a print statement."""
       self.eat('PRINT')
       self.eat('LPAREN')
       expression = self.parse_expression()
       self.eat('RPAREN')
       self.eat('SEMICOLON')
       return {"type": "print_statement", "expression": expression}
   def parse_for_statement(self):
       """Parses for loops for the Comp parser. Mirrors Parser.parse_for_statement."""
       self.eat('FOR')
       self.eat('LPAREN')
       if self.current_token and self.current_token.type in ('LET', 'CONST', 'VAR'):
           # Look ahead for an 'IN' token within the next few tokens to detect 'for ... in' syntax
           found_in = False
           for look_ahead in (1,2,3):
               if (self.token_index + look_ahead) < len(self.tokens) and self.tokens[self.token_index + look_ahead].type == 'IN':
                   found_in = True
                   break
           if found_in:
               decl_type = self.current_token.type
               self.advance()
               var_name = self.current_token.value
               self.eat('IDENTIFIER')
               self.eat('IN')
               iterable = self.parse_expression()
               self.eat('RPAREN')
               self.eat('LBRACE')
               body = []
               while self.current_token and self.current_token.type != 'RBRACE':
                   if self.current_token.type == 'PRINT':body.append(self.parse_print_statement())
                   elif self.current_token.type in ('CONST', 'LET'):body.append(self.parse_variable_declaration())
                   elif self.current_token.type in ('IDENTIFIER','MAIN'):
                       peek_token_idx = self.token_index + 1
                       if peek_token_idx < len(self.tokens) and self.tokens[peek_token_idx].type == 'EQUAL':body.append(self.parse_assignment_statement())
                       elif peek_token_idx < len(self.tokens) and self.tokens[peek_token_idx].type == 'DOT':
                           obj_name = self.current_token.value
                           self.eat('IDENTIFIER')
                           obj_expr = {"type": "variable_access", "name": obj_name}
                           expr = self.parse_member_access(obj_expr)
                           body.append(expr)
                           if expr["type"] == "member_access" and expr["is_call"]:self.eat('SEMICOLON')
                       elif peek_token_idx < len(self.tokens) and self.tokens[peek_token_idx].type == 'LPAREN':
                           expr = self.parse_function_call_expression()
                           body.append(expr)
                           self.eat('SEMICOLON')
                       else:raise Exception(f"Syntax Error: Unexpected token in for-body: {self.current_token.type if self.current_token else 'EOF'}")
                   elif self.current_token.type == 'RETURN':body.append(self.parse_return_statement())
                   elif self.current_token.type == 'FOR':body.append(self.parse_for_statement())
                   else:raise Exception(f"Syntax Error: Unexpected token in for-body: {self.current_token.type if self.current_token else 'EOF'}")
               self.eat('RBRACE')
               return {"type": "for_statement", "is_in": True, "decl_type": decl_type, "var_name": var_name, "iterable": iterable, "body": body}
       init = None
       if self.current_token and self.current_token.type in ('LET', 'CONST', 'VAR'):
           decl_type = self.current_token.type
           self.advance()
           var_name = self.current_token.value
           self.eat('IDENTIFIER')
           self.eat('EQUAL')
           init_value = self.parse_expression()
           init = {"type": "variable_declaration", "kind": decl_type, "name": var_name, "value": init_value}
       elif self.current_token and self.current_token.type != 'SEMICOLON':
           init = self.parse_expression()
       self.eat('SEMICOLON')
       condition = None
       if self.current_token and self.current_token.type != 'RPAREN':
           condition = self.parse_expression()
       self.eat('RPAREN')
       self.eat('LBRACE')
       body = []
       while self.current_token and self.current_token.type != 'RBRACE':
           if self.current_token.type == 'PRINT':body.append(self.parse_print_statement())
           elif self.current_token.type == 'FOR':body.append(self.parse_for_statement())
           elif self.current_token.type in ('CONST', 'LET'):body.append(self.parse_variable_declaration())
           elif self.current_token.type in ('IDENTIFIER','MAIN'):
               peek_token_idx = self.token_index + 1
               if peek_token_idx < len(self.tokens) and self.tokens[peek_token_idx].type == 'EQUAL':body.append(self.parse_assignment_statement())
               elif peek_token_idx < len(self.tokens) and self.tokens[peek_token_idx].type == 'DOT':
                   obj_name = self.current_token.value
                   self.eat('IDENTIFIER')
                   obj_expr = {"type": "variable_access", "name": obj_name}
                   expr = self.parse_member_access(obj_expr)
                   body.append(expr)
                   if expr["type"] == "member_access" and expr["is_call"]:self.eat('SEMICOLON')
               elif peek_token_idx < len(self.tokens) and self.tokens[peek_token_idx].type == 'LPAREN':
                   expr = self.parse_function_call_expression()
                   body.append(expr)
                   self.eat('SEMICOLON')
               else:raise Exception(f"Syntax Error: Unexpected token in for-body: {self.current_token.type if self.current_token else 'EOF'}")
           elif self.current_token.type == 'RETURN':body.append(self.parse_return_statement())
           elif self.current_token.type == 'FOR':body.append(self.parse_for_statement())
           else:raise Exception(f"Syntax Error: Unexpected token in for-body: {self.current_token.type if self.current_token else 'EOF'}")
       self.eat('RBRACE')
       return {"type": "for_statement", "is_in": False, "init": init, "condition": condition, "body": body}
   def parse_for_statement(self):
       """Parses for loops. Supports two forms:
       1) for ( <init>; <condition> ) { ... }
       2) for ( <var_decl> in <iterable> ) { ... }
       """
       self.eat('FOR')
       self.eat('LPAREN')
       # Detect for-in: e.g., for (let x in expr)
       if self.current_token and self.current_token.type in ('LET', 'CONST', 'VAR'):
           # Peek ahead to see if 'IN' occurs after identifier
           # Look ahead for an 'IN' token within the next few tokens to detect 'for ... in' syntax
           found_in = False
           for look_ahead in (1,2,3):
               if (self.token_index + look_ahead) < len(self.tokens) and self.tokens[self.token_index + look_ahead].type == 'IN':
                   found_in = True
                   break
           if found_in:
               decl_type = self.current_token.type
               self.advance()
               var_name = self.current_token.value
               self.eat('IDENTIFIER')
               self.eat('IN')
               iterable = self.parse_expression()
               self.eat('RPAREN')
               self.eat('LBRACE')
               body = []
               while self.current_token and self.current_token.type != 'RBRACE':
                   if self.current_token.type == 'PRINT':body.append(self.parse_print_statement())
                   elif self.current_token.type in ('CONST', 'LET'):body.append(self.parse_variable_declaration())
                   elif self.current_token.type in ('IDENTIFIER','MAIN'):
                       peek_token_idx = self.token_index + 1
                       if peek_token_idx < len(self.tokens) and self.tokens[peek_token_idx].type == 'EQUAL':body.append(self.parse_assignment_statement())
                       elif peek_token_idx < len(self.tokens) and self.tokens[peek_token_idx].type == 'DOT':
                           obj_name = self.current_token.value
                           self.eat('IDENTIFIER')
                           obj_expr = {"type": "variable_access", "name": obj_name}
                           expr = self.parse_member_access(obj_expr)
                           body.append(expr)
                           if expr["type"] == "member_access" and expr["is_call"]:self.eat('SEMICOLON')
                       elif peek_token_idx < len(self.tokens) and self.tokens[peek_token_idx].type == 'LPAREN':
                           expr = self.parse_function_call_expression()
                           body.append(expr)
                           self.eat('SEMICOLON')
                       else:raise Exception(f"Syntax Error: Unexpected token in for-body: {self.current_token.type if self.current_token else 'EOF'}")
                   elif self.current_token.type == 'RETURN':body.append(self.parse_return_statement())
                   elif self.current_token.type == 'FOR':body.append(self.parse_for_statement())
                   else:raise Exception(f"Syntax Error: Unexpected token in for-body: {self.current_token.type if self.current_token else 'EOF'}")
               self.eat('RBRACE')
               return {"type": "for_statement", "is_in": True, "decl_type": decl_type, "var_name": var_name, "iterable": iterable, "body": body}
       # Otherwise parse classic for (init; condition)
       init = None
       if self.current_token and self.current_token.type in ('LET', 'CONST', 'VAR'):
           # parse init without consuming trailing semicolon
           decl_type = self.current_token.type
           self.advance()
           var_name = self.current_token.value
           self.eat('IDENTIFIER')
           self.eat('EQUAL')
           init_value = self.parse_expression()
           init = {"type": "variable_declaration", "kind": decl_type, "name": var_name, "value": init_value}
       elif self.current_token and self.current_token.type != 'SEMICOLON':
           init = self.parse_expression()
       self.eat('SEMICOLON')
       condition = None
       if self.current_token and self.current_token.type != 'RPAREN':
           condition = self.parse_expression()
       self.eat('RPAREN')
       self.eat('LBRACE')
       body = []
       while self.current_token and self.current_token.type != 'RBRACE':
           if self.current_token.type == 'PRINT':body.append(self.parse_print_statement())
           elif self.current_token.type == 'FOR':body.append(self.parse_for_statement())
           elif self.current_token.type in ('CONST', 'LET'):body.append(self.parse_variable_declaration())
           elif self.current_token.type in ('IDENTIFIER','MAIN'):
               peek_token_idx = self.token_index + 1
               if peek_token_idx < len(self.tokens) and self.tokens[peek_token_idx].type == 'EQUAL':body.append(self.parse_assignment_statement())
               elif peek_token_idx < len(self.tokens) and self.tokens[peek_token_idx].type == 'DOT':
                   obj_name = self.current_token.value
                   self.eat('IDENTIFIER')
                   obj_expr = {"type": "variable_access", "name": obj_name}
                   expr = self.parse_member_access(obj_expr)
                   body.append(expr)
                   if expr["type"] == "member_access" and expr["is_call"]:self.eat('SEMICOLON')
               elif peek_token_idx < len(self.tokens) and self.tokens[peek_token_idx].type == 'LPAREN':
                   expr = self.parse_function_call_expression()
                   body.append(expr)
                   self.eat('SEMICOLON')
               else:raise Exception(f"Syntax Error: Unexpected token in for-body: {self.current_token.type if self.current_token else 'EOF'}")
           elif self.current_token.type == 'RETURN':body.append(self.parse_return_statement())
           elif self.current_token.type == 'FOR':body.append(self.parse_for_statement())
           else:raise Exception(f"Syntax Error: Unexpected token in for-body: {self.current_token.type if self.current_token else 'EOF'}")
       self.eat('RBRACE')
       return {"type": "for_statement", "is_in": False, "init": init, "condition": condition, "body": body}
   def parse_input_statement(self):
       self.eat('READLN')
       self.eat('LPAREN')
       prompt=self.parse_expression()["value"]
       val=input(prompt)
       self.eat('RPAREN')
       return {"type":"literal","data_type":'string',"value":val}
   # --- Expression Parsing with Operator Precedence ---
   # Implements operator precedence using a recursive descent parser.
   # Grammar for expressions (simplified):
   # expression     : comparison ( ( '==' | '!=' | '<' | '>' | '<=' | '>=' ) comparison )*
   # comparison     : term ( ( '+' | '-' ) term )*
   # term           : factor ( ( '*' | '/' | '%' ) factor )*
   # factor         : ( '+' | '-' )* primary
   # primary        : NUMBER | STRING_LITERAL | IDENTIFIER | BACKTICK_STRING | NEW_EXPRESSION | LPAREN expression RPAREN | function_call | member_access
   def parse_expression(self):
       """Parses comparison operators (==, !=, <, >, <=, >=)."""
       left = self._parse_term()
       while self.current_token and self.current_token.type in ('EQUAL_EQUAL', 'NOT_EQUAL', 'LESS_THAN', 'GREATER_THAN', 'LESS_EQUAL', 'GREATER_EQUAL'):
           operator = self.current_token.value
           self.advance()
           right = self._parse_term()
           left = {"type": "binary_op", "operator": operator, "left": left, "right": right}
       return left
   def _parse_term(self):
       """Parses addition and subtraction operators (+, -)."""
       left = self._parse_factor()
       while self.current_token and self.current_token.type in ('PLUS', 'MINUS'):
           operator = self.current_token.value
           self.advance()
           right = self._parse_factor()
           left = {"type": "binary_op", "operator": operator, "left": left, "right": right}
       return left
   def _parse_factor(self):
       """Parses multiplication, division, and modulo operators (*, /, %)."""
       # Handle unary plus/minus (e.g., -5, +variable)
       if self.current_token and self.current_token.type in ('PLUS', 'MINUS'):
           operator = self.current_token.value
           self.advance()
           operand = self._parse_primary_expression()
           return {"type": "unary_op", "operator": operator, "operand": operand}
       left = self._parse_primary_expression()
       while self.current_token and self.current_token.type in ('MULTIPLY', 'DIVIDE', 'MODULO'):
           operator = self.current_token.value
           self.advance()
           right = self._parse_primary_expression()
           left = {"type": "binary_op", "operator": operator, "left": left, "right": right}
       return left
   def _parse_primary_expression(self):
       """Parses the most basic expressions: literals, identifiers, new expressions, parenthesized expressions."""
       if self.current_token.type == 'STRING_LITERAL':
           value = self.current_token.value
           self.eat('STRING_LITERAL')
           return {"type": "literal", "value": value, "data_type": "string"}
       elif self.current_token.type == 'NUMBER':
           value = self.current_token.value
           self.eat('NUMBER')
           return {"type": "literal", "value": int(value), "data_type": "number"}
       elif self.current_token.type == 'BACKTICK_STRING':
           template_parts = []
           string_content = self.current_token.value
           self.eat('BACKTICK_STRING')
           parts = re.split(r'\$\{(\w+)\}', string_content) # Splits by ${variableName}
           for i, part in enumerate(parts):
               if i % 2 == 0: # Even indices are literal strings
                   if part:template_parts.append({"type": "literal", "value": part, "data_type": "string"})
               else:template_parts.append({"type": "variable_access", "name": part})
           return {"type": "template_string", "parts": template_parts}
       elif self.current_token.type in ('IDENTIFIER','MAIN'):
           # This could be a variable access, function call, or start of a member access
           name = self.current_token.value
           self.advance() # Consume the identifier first
           next_token = self.current_token # Now current_token is what was next after the identifier
           if next_token and next_token.type == 'LPAREN':
               # It's a function call (e.g., myFunction(arg))
               # Re-create the function call node with the already consumed identifier
               func_call_node = {"type": "function_call", "name": name, "arguments": []}
               self.eat('LPAREN')
               if self.current_token and self.current_token.type != 'RPAREN':
                   func_call_node["arguments"].append(self.parse_expression())
                   while self.current_token and self.current_token.type == 'COMMA':self.eat('COMMA');func_call_node["arguments"].append(self.parse_expression())
               self.eat('RPAREN')
               return func_call_node
           elif next_token and next_token.type == 'DOT':
               # It's a member access (e.g., obj.field or obj.method())
               obj_expr = {"type": "variable_access", "name": name}
               return self.parse_member_access(obj_expr)
           else:return {"type": "variable_access", "name": name}
       elif self.current_token.type=='READLN':
           return self.parse_input_statement()
       elif self.current_token.type == 'NEW':return self.parse_new_expression()
       elif self.current_token.type == 'LPAREN': # For parenthesized expressions (e.g., (2 + 3) * 4)
           self.eat('LPAREN')
           expr = self.parse_expression() # Recursively parse the inner expression
           self.eat('RPAREN')
           return expr
       else:raise Exception(f"Syntax Error: Unexpected token in primary expression: {self.current_token.type if self.current_token else 'EOF'}")
   def parse_function_call_expression(self):
       """
       Parses a function call expression.
       Note: This is called when the identifier *and* LPAREN are already peeked.
       """
       function_name = self.current_token.value
       if self.current_token.type == 'IDENTIFIER':
           self.eat('IDENTIFIER')
       elif self.current_token.type == 'MAIN':
           self.eat('MAIN')
       else:
           raise Exception(f"Syntax Error: Expected function name, but got {self.current_token.type if self.current_token else 'EOF'}")
       self.eat('LPAREN')
       arguments = []
       if self.current_token and self.current_token.type != 'RPAREN':
           arguments.append(self.parse_expression())
           while self.current_token and self.current_token.type == 'COMMA':self.eat('COMMA');arguments.append(self.parse_expression())
       self.eat('RPAREN')
       return {"type": "function_call", "name": function_name, "arguments": arguments}
   def parse_member_access(self, obj_expr):
       """
       Parses member access (obj.field) or method calls (obj.method())
       or member assignments (obj.field = value).
       """
       self.eat('DOT')
       member_name = self.current_token.value
       self.eat('IDENTIFIER')
       is_call = False
       member_args = []
       # Check if it's a method call (e.g., obj.method())
       if self.current_token and self.current_token.type == 'LPAREN':
           is_call = True
           self.eat('LPAREN')
           if self.current_token and self.current_token.type != 'RPAREN':
               member_args.append(self.parse_expression())
               while self.current_token and self.current_token.type == 'COMMA':self.eat('COMMA');member_args.append(self.parse_expression())
           self.eat('RPAREN')
       # Check for assignment to a member (e.g., obj.property = value;)
       if self.current_token and self.current_token.type == 'EQUAL':
           self.eat('EQUAL')
           value_expr = self.parse_expression()
           self.eat('SEMICOLON') # Member assignment consumes its own semicolon
           return {"type": "member_assignment", "object": obj_expr, "member": member_name, "value": value_expr}
       # If it's not an assignment or a call, it's just an access to a property
       return {"type": "member_access", "object": obj_expr, "member": member_name, "is_call": is_call, "arguments": member_args}
   def parse_variable_declaration(self):
       """Parses a variable declaration (e.g., let x = 10; or const PI = 3.14;)."""
       declaration_type = self.current_token.type
       self.advance() # Consume CONST,LET or var
       variable_name = self.current_token.value
       self.eat('IDENTIFIER')
       self.eat('EQUAL')
       value_expression = self.parse_expression()
       self.eat('SEMICOLON')
       return {"type": "variable_declaration", "kind": declaration_type, "name": variable_name, "value": value_expression}
   def parse_assignment_statement(self):
       """Parses a variable assignment statement (e.g., x = 20;)."""
       variable_name = self.current_token.value
       self.eat('IDENTIFIER')
       self.eat('EQUAL')
       value_expression = self.parse_expression()
       self.eat('SEMICOLON')
       return {"type": "assignment_statement", "name": variable_name, "value": value_expression}
   def parse_new_expression(self):
       """Parses an object creation expression (e.g., new MyClass(arg1, arg2))."""
       self.eat('NEW')
       class_name = self.current_token.value
       self.eat('IDENTIFIER')
       self.eat('LPAREN')
       arguments = []
       if self.current_token and self.current_token.type != 'RPAREN':
           arguments.append(self.parse_expression())
           while self.current_token and self.current_token.type == 'COMMA':self.eat('COMMA');arguments.append(self.parse_expression())
       self.eat('RPAREN')
       return {"type": "object_creation", "class_name": class_name, "arguments": arguments}
   def parse(self):
       """Starts the parsing process for the entire program."""
       return self.parse_main()
class Lexer:
   """Converts source code text into a stream of tokens."""
   def __init__(self, text):
       self.text = self.remove_comments(text)
       self.pos = 0
       self.current_char = self.text[self.pos] if len(self.text) > 0 else None
   def remove_comments(self, text):
       """Removes single-line (//) and multi-line (/* ... */) comments."""
       text = re.sub(r'//.*', '', text)
       text = re.sub(r'/\*.*?\*/', '', text, flags=re.DOTALL)
       return text
   def advance(self):
       """Moves the lexer's position to the next character."""
       self.pos += 1
       if self.pos < len(self.text):self.current_char = self.text[self.pos]
       else:self.current_char = None
   def skip_whitespace(self):
       """Skips over whitespace characters."""
       while self.current_char is not None and self.current_char.isspace():self.advance()
   def identifier(self):
       """Reads an identifier (alphanumeric and underscores)."""
       result = ''
       while self.current_char is not None and (self.current_char.isalnum() or self.current_char == '_'):result += self.current_char;self.advance()
       return result
   def string_literal(self):
       """Reads a double-quoted string literal."""
       result = ''
       self.advance() # Consume the opening '"'
       while self.current_char is not None and self.current_char != '"' and self.current_char != "'":result += self.current_char;self.advance()
       if self.current_char == '"' or self.current_char=="'":self.advance();return result
       else:raise Exception("Lexer Error: Unterminated string literal")
   def backtick_string_literal(self):
       """Reads a backtick-quoted string literal (for template strings)."""
       result = ''
       self.advance() # Consume the opening '`'
       while self.current_char is not None and self.current_char != '`':result += self.current_char;self.advance()
       if self.current_char == '`':self.advance(); return result
       else:raise Exception("Lexer Error: Unterminated backtick string literal")
   def peek(self, offset=1):
       """Peeks at a character without advancing the lexer's position."""
       peek_pos = self.pos + offset
       if peek_pos < len(self.text):return self.text[peek_pos];return None
   def get_next_token(self):
       """Returns the next token from the input text."""
       while self.current_char is not None:
           self.skip_whitespace()
           if self.current_char is None:return None # Reached end of input after skipping whitespace
           if self.current_char.isalpha() or self.current_char == '_':
               identifier = self.identifier()
               # Keywords
               if identifier == 'public': return Token('PUBLIC', 'public')
               elif identifier == 'private': return Token('PRIVATE', 'private')
               elif identifier == 'class': return Token('CLASS', 'class')
               elif identifier == 'main': return Token('MAIN', 'main') # 'main' as a specific keyword/identifier
               elif identifier == 'self': return Token('SELF', 'self')
               elif identifier == 'inner_self' or identifier=='innerSelf': return Token('INNERSELF', 'inner_self')
               elif identifier == 'func': return Token('DEF', 'def')
               elif identifier == 'println': return Token('PRINT', 'print')
               elif identifier == 'pass': return Token('PASS', 'pass')
               elif identifier == 'let': return Token('LET', 'let')
               elif identifier == 'const': return Token('CONST', 'const')
               elif identifier == 'var': return Token('VAR','var')
               elif identifier == 'new': return Token('NEW', 'new')
               elif identifier == 'readln': return Token('READLN', 'readln')
               elif identifier == 'return': return Token('RETURN', 'return')
               elif identifier == 'for': return Token('FOR', 'for')
               elif identifier == 'in': return Token('IN', 'in')
               else: return Token('IDENTIFIER', identifier)
           # Arithmetic Operators
           if self.current_char == '+': self.advance(); return Token('PLUS', '+')
           if self.current_char == '-': self.advance(); return Token('MINUS', '-')
           if self.current_char == '*': self.advance(); return Token('MULTIPLY', '*')
           if self.current_char == '/': self.advance(); return Token('DIVIDE', '/')
           if self.current_char == '%': self.advance(); return Token('MODULO', '%')
           # Comparison Operators
           if self.current_char == '=':
               if self.peek() == '=':
                   self.advance()
                   self.advance()
                   return Token('EQUAL_EQUAL', '==')
               else:self.advance();return Token('EQUAL', '=')
           if self.current_char == '!':
               if self.peek() == '=':
                   self.advance()
                   self.advance()
                   return Token('NOT_EQUAL', '!=')
               else:raise Exception(f"Lexer Error: Invalid character: {self.current_char}")
           if self.current_char == '<':
               if self.peek() == '=':
                   self.advance()
                   self.advance()
                   return Token('LESS_EQUAL', '<=')
               else:self.advance();return Token('LESS_THAN', '<')
           if self.current_char == '>':
               if self.peek() == '=':
                   self.advance()
                   self.advance()
                   return Token('GREATER_EQUAL', '>=')
               else:self.advance();return Token('GREATER_THAN', '>')
           # Other Punctuation
           if self.current_char == '(': self.advance(); return Token('LPAREN', '(')
           if self.current_char == ')': self.advance(); return Token('RPAREN', ')')
           if self.current_char == '{': self.advance(); return Token('LBRACE', '{')
           if self.current_char == '}': self.advance(); return Token('RBRACE', '}')
           if self.current_char == ':': self.advance(); return Token('COLON', ':')
           if self.current_char == ';': self.advance(); return Token('SEMICOLON', ';')
           if self.current_char == '"': return Token('STRING_LITERAL', self.string_literal())
           if self.current_char == "'": return Token('STRING_LITERAL', self.string_literal())
           if self.current_char == '`': return Token('BACKTICK_STRING', self.backtick_string_literal())
           if self.current_char=="@": self.advance(); return Token('AT',"@")
           if self.current_char == ',': self.advance(); return Token('COMMA', ',')
           if self.current_char == '.': self.advance(); return Token('DOT', '.')
           if self.current_char.isdigit():
               number = ''
               while self.current_char is not None and self.current_char.isdigit():number += self.current_char;self.advance()
               return Token('NUMBER', number)
           raise Exception(f"Lexer Error: Invalid character: {self.current_char}")
       return None # End of file token
   def tokenize(self):
       """Tokenizes the entire input text."""
       tokens = []
       token = self.get_next_token()
       while token:tokens.append(token);token = self.get_next_token()
       return tokens
# Custom exception for returning from functions
class FunctionReturn(Exception):
 def __init__(self, value):self.value = value
# Represents an instance of a PayJar class at runtime
class PayJarObject:
   """Represents an object instance of a user-defined class."""
   def __init__(self, class_name, fields, methods):
       self.class_name = class_name
       self.fields = fields # Dictionary: {"field_name": {"value": val, "kind": "let/const"}}
       self.methods = methods # Dictionary: {"method_name": function_definition_node}
   def __repr__(self):field_str = ', '.join(f"{name}={data['value']}" for name, data in self.fields.items());return f"<Object {self.class_name} ({field_str})>"
class Interpreter:
   """Interprets the Abstract Syntax Tree (AST) generated by the parser."""
   def __init__(self):
       self.scopes = [{}] # List of dictionaries, representing scope stack
       self.functions = {} # Global functions
       self.user_defined_classes = {} # User-defined class definitions
       
   @property
   def current_scope(self):return self.scopes[-1]
   def push_scope(self, scope_dict=None):
       """Adds a new scope to the scope stack."""
       if scope_dict is None:scope_dict = {}
       self.scopes.append(scope_dict)
   def pop_scope(self):
       """Removes the top scope from the scope stack."""
       if len(self.scopes) > 1:self.scopes.pop()
       else:raise Exception("Runtime Error: Cannot pop global scope.")
   def get_variable(self, name):
       """Retrieves a variable's data (value and kind) from the current scope stack."""
       for scope in reversed(self.scopes):
           if name in scope:return scope[name]
       raise Exception(f"Runtime Error: Undefined variable '{name}'")
   def set_variable(self, name, value, kind=None, declare_if_not_exists=False):
       """
       Sets or declares a variable.
       If 'kind' is provided, it's a declaration.
       If 'declare_if_not_exists' is True, it declares if not found in any scope.
       """
       if kind: # This is a declaration
           if name in self.current_scope:raise Exception(f"Runtime Error: Redeclaration of variable '{name}' is not allowed in this scope.")
           self.current_scope[name] = {"value": value, "kind": kind}
       else: # This is an assignment
           for scope in reversed(self.scopes):
               if name in scope:
                   if scope[name]["kind"] == 'CONST':raise Exception(f"Runtime Error: Cannot assign to a constant variable '{name}'.")
                   scope[name]["value"] = value
                   return scope[name]['value']
           if declare_if_not_exists:self.current_scope[name] = {"value": value, "kind": "LET"} # Default to 'let' if declaring on assignment
           else:raise Exception(f"Runtime Error: Assignment to undefined variable '{name}'.")
   def interpret(self, ast):
       """Starts the interpretation process from the main AST node."""
       if ast["type"] == "main_definition":
           # First pass: Register functions and classes (hoisting)
           for statement in ast["body"]:
               if statement["type"] == "function_definition":self.visit(statement)
               elif statement["type"] == "class_definition":self.visit(statement)
           # Second pass: Execute statements in main body
           for statement in ast["body"]:
               if statement["type"] not in ("function_definition", "class_definition"):
                   try:self.visit(statement)
                   except FunctionReturn as e:print(f"Warning: Return statement encountered in main body. Value: {e.value}")
       else:raise Exception(f"Runtime Error: Unsupported AST type for interpretation: {ast['type']}")
   def visit(self, node, current_instance=None):
       """
       Generic visit method to dispatch to specific handlers based on node type.
       'current_instance' is used for method calls and member access on objects.
       """
       node_type = node["type"]
       if node_type == "print_statement":self.visit_print_statement(node)
       elif node_type == "variable_declaration":self.visit_variable_declaration(node)
       elif node_type == "assignment_statement":self.visit_assignment_statement(node)
       elif node_type == "literal":return node["value"]
       elif node_type == "variable_access":return self.visit_variable_access(node)
       elif node_type == "template_string":
           return self.visit_template_string(node)
       elif node_type == "function_definition":
           self.visit_function_definition(node)
       elif node_type == "function_call":return self.visit_function_call(node, current_instance)
       elif node_type == "return_statement":self.visit_return_statement(node)
       elif node_type == "class_definition":self.visit_class_definition(node)
       elif node_type == "object_creation":return self.visit_object_creation(node)
       elif node_type == "member_access":return self.visit_member_access(node, current_instance)
       elif node_type == "member_assignment":self.visit_member_assignment(node, current_instance)
       elif node_type == "binary_op":return self.visit_binary_op(node)
       elif node_type == "unary_op":return self.visit_unary_op(node)
       elif node_type == "field_declaration":pass # Field declarations are handled during object creation
       elif node_type == "for_statement":return self.visit_for_statement(node, current_instance)
       else:raise Exception(f"Runtime Error: Unknown AST node type: {node_type}")
   def visit_print_statement(self, node):
       """Executes a print statement."""
       value_to_print = self.visit(node["expression"])
       print(value_to_print)
   def visit_input_statement(self,node):
       return input(node["prompt"]["value"])
   def visit_variable_declaration(self, node):
       """Executes a variable declaration."""
       var_name = node["name"]
       var_kind = node["kind"]
       var_value = self.visit(node["value"])
       self.set_variable(var_name, var_value, kind=var_kind)
   def visit_assignment_statement(self, node):
       """Executes a variable assignment."""
       var_name = node["name"]
       new_value = self.visit(node["value"])
       self.set_variable(var_name, new_value)
   def visit_variable_access(self, node):
       """Retrieves the value of a variable."""
       var_name = node["name"]
       return self.get_variable(var_name)["value"]
   def visit_template_string(self, node):
       """Evaluates a template string (e.g., `Hello, ${name}!`)."""
       result_string = ""
       for part in node["parts"]:
           if part["type"] == "literal":result_string += part["value"]
           elif part["type"] == "variable_access":variable_value = self.get_variable(part["name"])["value"];result_string += str(variable_value)
           else:raise Exception(f"Runtime Error: Unexpected part in template string: {part['type']}")
       return result_string
   def visit_function_definition(self, node):
       """Registers a function definition (global or method)."""
       func_name = node["name"]
       if not node.get('is_method', False) and func_name in self.functions:raise Exception(f"Runtime Error: Function '{func_name}' already defined globally.")
       self.functions[func_name] = node
   def visit_function_call(self, node, current_instance=None):
       """Executes a function or method call."""
       func_name = node["name"]
       func_definition = None
       # Prioritize method call if an instance is provided
       if current_instance and func_name in current_instance.methods:func_definition = current_instance.methods[func_name]
       elif func_name in self.functions:func_definition = self.functions[func_name]
       else:raise Exception(f"Runtime Error: Call to undefined function or method '{func_name}'")
       expected_params = func_definition["parameters"]
       provided_args_nodes = node["arguments"]
       # Adjust expected arguments count if it's a method (due to implicit 'self')
       if func_definition.get('is_method') and expected_params and expected_params[0] == 'self':expected_args_count = len(expected_params) - 1
       else:expected_args_count = len(expected_params)
       if expected_args_count != len(provided_args_nodes):raise Exception(f"Runtime Error: Function/Method '{func_name}' expected {expected_args_count} arguments but got {len(provided_args_nodes)}")
       self.push_scope() # Create a new scope for function execution
       # Bind 'self' for methods
       if func_definition.get('is_method') and current_instance:self.set_variable('self', current_instance, kind='LET');param_offset = 1 # Skip 'self' when binding other parameters
       else:param_offset = 0
       evaluated_args = [self.visit(arg_node, current_instance) for arg_node in provided_args_nodes]
       # Bind arguments to parameters in the new scope
       for i, param_name in enumerate(expected_params[param_offset:]):self.set_variable(param_name, evaluated_args[i], kind='LET')
       return_value = None
       try:
           for statement in func_definition["body"]:self.visit(statement, current_instance) # Pass current_instance down to nested calls
       except FunctionReturn as e:return_value = e.value
       finally:self.pop_scope() # Always pop the scope when function exits
       return return_value
   def visit_return_statement(self, node):
       """Handles a return statement by raising a custom exception."""
       return_value = self.visit(node["expression"])
       raise FunctionReturn(return_value)
   def visit_class_definition(self, node):
       """Registers a class definition."""
       class_name = node["name"]
       if class_name in self.user_defined_classes:raise Exception(f"Runtime Error: Class '{class_name}' already defined.")
       self.user_defined_classes[class_name] = node
   def visit_object_creation(self, node):
       """Creates a new instance of a class."""
       class_name = node["class_name"]
       evaluated_args = [self.visit(arg_node) for arg_node in node["arguments"]]
       if class_name in self.user_defined_classes:return self.user_defined_classes[class_name](evaluated_args)
       if class_name not in self.user_defined_classes:raise Exception(f"Runtime Error: Attempt to create instance of undefined class '{class_name}'")
       class_definition = self.user_defined_classes[class_name]
       instance_fields = {}
       instance_methods = {}
       # Initialize fields with their default values
       for field in class_definition["fields"]:
           field_name = field["name"]
           field_kind = field["kind"]
           initial_value = None
           if field["value"]:initial_value = self.visit(field["value"])
           instance_fields[field_name] = {"value": initial_value, "kind": field_kind}
       # Map method definitions to the instance
       for method_node in class_definition["methods"]:method_node["is_method"] = True;instance_methods[method_node["name"]] = method_node
       instance = PayJarObject(class_name, instance_fields, instance_methods)
       # Execute the constructor if it exists
       if class_definition["constructor"]:
           constructor_node = class_definition["constructor"]
           constructor_node["is_method"] = True # Ensure constructor is treated as a method
           expected_constructor_params = constructor_node["parameters"]
           if not expected_constructor_params or expected_constructor_params[0] != 'self':raise Exception(f"Runtime Error: Constructor 'init' for class '{class_name}' must have 'self' as its first parameter.")
           if (len(expected_constructor_params) - 1) != len(evaluated_args):raise Exception(f"Runtime Error: Constructor for '{class_name}' expected {len(expected_constructor_params) - 1} arguments but got {len(evaluated_args)}")
           self.push_scope() # New scope for constructor execution
           self.set_variable('self', instance, kind='LET') # Bind 'self' to the new instance
           for i, param_name in enumerate(expected_constructor_params[1:]):self.set_variable(param_name, evaluated_args[i], kind='LET')
           try:
               for statement in constructor_node["body"]:self.visit(statement, instance) # Pass instance to constructor body
           except FunctionReturn:pass # Constructors don't typically return values
           finally:self.pop_scope() # Pop constructor's scope
       return instance
   def visit_member_access(self, node, current_instance=None):
       """Handles accessing a member (field or method) of an object."""
       obj = self.visit(node["object"]) # Evaluate the object expression
       if not isinstance(obj, PayJarObject):raise Exception(f"Runtime Error: Attempt to access member '{node['member']}' on a non-object type: {obj}")
       member_name = node["member"]
       if node["is_call"]: # It's a method call
           if member_name not in obj.methods:raise Exception(f"Runtime Error: Method '{member_name}' not found on object of type '{obj.class_name}'")
           # Directly call the method using visit_function_call, passing the object as current_instance
           return self.visit_function_call({"type": "function_call", "name": member_name, "arguments": node["arguments"]},current_instance=obj)
       else: # It's a field access
           if member_name not in obj.fields:raise Exception(f"Runtime Error: Field '{member_name}' not found on object of type '{obj.class_name}'");return obj.fields[member_name]["value"]
   def visit_member_assignment(self, node, current_instance=None):
       """Handles assigning a value to an object's field."""
       obj = self.visit(node["object"]) # Evaluate the object expression
       if not isinstance(obj, PayJarObject):raise Exception(f"Runtime Error: Attempt to assign member '{node['member']}' on a non-object type: {obj}")
       member_name = node["member"]
       new_value = self.visit(node["value"])
       if member_name not in obj.fields:raise Exception(f"Runtime Error: Field '{member_name}' not found on object of type '{obj.class_name}' for assignment.")
       if obj.fields[member_name]["kind"] == 'CONST':raise Exception(f"Runtime Error: Cannot assign to constant field '{member_name}' of object '{obj.class_name}'.")
       obj.fields[member_name]["value"] = new_value
   def visit_for_statement(self, node, current_instance=None):
       """Executes a for loop AST node. Supports both for-in and for(init;condition)."""
       if node.get('is_in'):
           # Evaluate iterable: support range(...) specially
           iterable_node = node['iterable']
           if isinstance(iterable_node, dict) and iterable_node.get('type') == 'function_call' and iterable_node.get('name') == 'range':
               args = [self.visit(arg) for arg in iterable_node.get('arguments', [])]
               if len(args) == 1: rng = range(args[0])
               elif len(args) == 2: rng = range(args[0], args[1])
               elif len(args) == 3: rng = range(args[0], args[1], args[2])
               else: raise Exception("Runtime Error: range() expects 1-3 arguments")
               iterable = list(rng)
           else:
               iterable = self.visit(iterable_node)
           # Iterate
           for item in iterable:
               self.push_scope()
               # declare loop variable
               decl_kind = node.get('decl_type', 'LET')
               self.set_variable(node['var_name'], item, kind=decl_kind)
               try:
                   for stmt in node.get('body', []):
                       self.visit(stmt, current_instance)
               except FunctionReturn as e:
                   self.pop_scope()
                   raise
               finally:
                   self.pop_scope()
           return None
       # Classic for(init; condition)
       self.push_scope()
       try:
           init = node.get('init')
           if init:
               if init.get('type') == 'variable_declaration':
                   self.set_variable(init['name'], self.visit(init['value']), kind=init['kind'])
               else:
                   # expression init
                   self.visit(init)
           while True:
               cond = node.get('condition')
               if cond is not None:
                   cond_val = self.visit(cond)
                   if not cond_val:break
               # No condition means infinite loop
               try:
                   for stmt in node.get('body', []):
                       self.visit(stmt, current_instance)
               except FunctionReturn as e:
                   raise
           return None
       finally:
           self.pop_scope()
   def visit_binary_op(self, node):
       """Executes binary operations (e.g., +, -, *, /, %, ==, !=, <, >, <=, >=)."""
       left_val = self.visit(node["left"])
       right_val = self.visit(node["right"])
       operator = node["operator"]
       # Type checking and operation execution
       if operator == '+':return left_val + right_val
       elif operator == '-':return left_val - right_val
       elif operator == '*':return left_val * right_val
       elif operator == '/':
           if right_val == 0:raise Exception("Runtime Error: Division by zero.")
           # Ensure integer division if both operands are integers
           if isinstance(left_val, int) and isinstance(right_val, int):return left_val // right_val;return left_val / right_val
       elif operator == '%':
           if right_val == 0:raise Exception("Runtime Error: Modulo by zero.");return left_val % right_val
       elif operator == '//':
           if right_val==0:raise Exception("Runtimme Error: Division by zero")
       elif operator == '==':return left_val == right_val
       elif operator == '!=':return left_val != right_val
       elif operator == '<':return left_val < right_val
       elif operator == '>':return left_val > right_val
       elif operator == '<=':return left_val <= right_val
       elif operator == '>=':return left_val >= right_val
       else:raise Exception(f"Runtime Error: Unsupported binary operator: {operator}")
   def visit_unary_op(self, node):
       """Executes unary operations (e.g., +x, -y)."""
       operand_val = self.visit(node["operand"])
       operator = node["operator"]
       if operator == '+':return +operand_val
       elif operator == '-':return -operand_val
       else:raise Exception(f"Runtime Error: Unsupported unary operator: {operator}")
class Comp(Parser):
   """Parses a list of tokens into an Abstract Syntax Tree (AST)."""
   def __init__(self, tokens,textin):
       self.tokens = tokens
       self.current_token = None
       self.token_index = -1
       self.text=textin
       self.advance()
   def parse_print_statement(self):
        "print that returns a string instead of printing"
        self.eat('PRINT')
        self.eat('LPAREN')
        expr = self.parse_expression()
        self.eat('RPAREN')
        self.eat('SEMICOLON')
        return {"type": "print_statement", "expression": expr}
   def parse_input_statement(self):
       """Parses an input statement (readln)."""
       self.eat('READLN')
       self.eat('LPAREN')
       prompt_expr = super().parse_expression()
       self.eat('RPAREN')
       self.eat('SEMICOLON')
       return {"type": "input_statement", "prompt": prompt_expr}
class PJRT:
   """Main Tkinter application for the Esolang Comp."""
   def __init__(self, code,debug=False):
       self.code=code
       self.run_code(debug)
   def run_code(self,debug_on):
       """uhm"""
       tokens=[]
       ast={}
       try:
           lexer = Lexer(self.code)
           tokens = lexer.tokenize()
           if debug_on:print("Lexer Tokens:", tokens)
           parser = Parser(tokens)
           ast = parser.parse()
           if debug_on:print("Parsed AST:", ast)
           interpreter = Interpreter() # Create an instance of the interpreter
           interpreter.interpret(ast) # Run the interpretation
           
       except Exception as e:print(f"Error during PJRT: {e}")
       return tokens,ast
def PJS(code, textin):
    """Functional runtime entrypoint.

    Parses `code`, runs the interpreter while capturing printed output,
    and returns a deterministic string containing the captured output
    and the parsed `Comp` result for the provided `textin`.
    """
    from io import StringIO
    import contextlib

    try:
        # Lex + parse to AST
        lexer = Lexer(code)
        tokens = lexer.tokenize()
        parser = Parser(tokens)
        ast = parser.parse()

        # Build Comp result (uses text input for readln semantics)
        comp = Comp(tokens, textin)
        comp_re = comp.parse()

        # Run interpreter and capture printed output
        interpreter = Interpreter()
        buf = StringIO()
        with contextlib.redirect_stdout(buf):
            interpreter.interpret(ast)
        output = buf.getvalue()

        # Return a deterministic string for the UI
        return f"Output:{output}\nCompParse:{repr(comp_re)}"
    except Exception:
        # Re-raise so callers (UI) can handle and present errors
        raise