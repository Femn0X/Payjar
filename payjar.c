// pjc.c - PayJar (PJ) Esolang Interpreter in C
// Language: public class main(@self){...}, let/const/var, println, readln,
// func, class, new, if/else, while, template strings, +/-/*/div/%, comparisons

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <ctype.h>

// Forward declare ScopeTable for VarEntry
typedef struct ScopeTable ScopeTable;

/* ============================================================
 * FORWARD DECLARATIONS
 * ============================================================ */

typedef struct Value Value;
typedef struct AstNode AstNode;
typedef struct Interpreter Interpreter;
typedef struct Scope Scope;
typedef struct PJObject PJObject;
typedef struct ClassDef ClassDef;
typedef struct FuncDef FuncDef;

/* ============================================================
 * TOKENS
 * ============================================================ */

typedef enum {
    TOK_PUBLIC, TOK_PRIVATE, TOK_CLASS, TOK_MAIN, TOK_SELF, TOK_INNERSELF,
    TOK_DEF, TOK_PRINT, TOK_LET, TOK_CONST, TOK_VAR, TOK_NEW,
    TOK_READLN,TOK_READI,TOK_READF, TOK_RETURN, TOK_IF, TOK_ELSE, TOK_WHILE, TOK_FOR, TOK_IN, TOK_RANGE,
    TOK_PACKAGE,
    TOK_IDENTIFIER, TOK_NUMBER, TOK_STRING_LITERAL, TOK_BACKTICK_STRING,
    TOK_PLUS, TOK_MINUS, TOK_MULTIPLY, TOK_DIVIDE, TOK_MODULO,
    TOK_EQUAL, TOK_EQUAL_EQUAL, TOK_NOT_EQUAL,
    TOK_LESS_THAN, TOK_GREATER_THAN, TOK_LESS_EQUAL, TOK_GREATER_EQUAL,
    TOK_LPAREN, TOK_RPAREN,TOK_LBRACK,TOK_RBRACK, TOK_LBRACE, TOK_RBRACE,
    TOK_SEMICOLON, TOK_COMMA, TOK_DOT, TOK_COLON, TOK_AT,
    TOK_EOF
} TokenType;

typedef struct {
    TokenType type;
    char *value; /* heap-allocated string */
} Token;

/* ============================================================
 * LEXER
 * ============================================================ */

typedef struct {
    char *src;
    int pos;
    int len;
} Lexer;

static char *strdup_n(const char *s, int n) {
    char *r = malloc(n + 1);
    memcpy(r, s, n);
    r[n] = '\0';
    return r;
}

static char *make_token_val(const char *s) {
    return strdup(s);
}

static void lexer_advance(Lexer *l) { l->pos++; }
static char lexer_cur(Lexer *l) {
    return (l->pos < l->len) ? l->src[l->pos] : '\0';
}
static char lexer_peek(Lexer *l) {
    return (l->pos + 1 < l->len) ? l->src[l->pos + 1] : '\0';
}

// Strip // and block comments into a new buffer
static char *remove_comments(const char *src) {
    int n = strlen(src);
    char *out = malloc(n + 1);
    int i = 0, j = 0;
    while (i < n) {
        if (i + 1 < n && src[i] == '/' && src[i+1] == '/') {
            while (i < n && src[i] != '\n') i++;
        } else if (i + 1 < n && src[i] == '/' && src[i+1] == '*') {
            i += 2;
            while (i + 1 < n && !(src[i] == '*' && src[i+1] == '/')) i++;
            i += 2;
        } else {
            out[j++] = src[i++];
        }
    }
    out[j] = '\0';
    return out;
}

static Token make_tok(TokenType t, const char *v) {
    Token tok;
    tok.type = t;
    tok.value = make_token_val(v);
    return tok;
}

static Token lexer_next(Lexer *l) {
    /* skip whitespace */
    while (l->pos < l->len && isspace((unsigned char)lexer_cur(l)))
        lexer_advance(l);
    if (l->pos >= l->len) return make_tok(TOK_EOF, "");

    char c = lexer_cur(l);

    /* identifiers / keywords */
    if (isalpha((unsigned char)c) || c == '_') {
        int start = l->pos;
        while (l->pos < l->len && (isalnum((unsigned char)lexer_cur(l)) || lexer_cur(l) == '_'))
            lexer_advance(l);
        int len = l->pos - start;
        char *id = strdup_n(l->src + start, len);

        TokenType tt = TOK_IDENTIFIER;
        if      (!strcmp(id,"public"))   tt = TOK_PUBLIC;
        else if (!strcmp(id,"private"))  tt = TOK_PRIVATE;
        else if (!strcmp(id,"class"))    tt = TOK_CLASS;
        else if (!strcmp(id,"main"))     tt = TOK_MAIN;
        else if (!strcmp(id,"self"))     tt = TOK_SELF;
        else if (!strcmp(id,"inner_self") || !strcmp(id,"innerSelf")) tt = TOK_INNERSELF;
        else if (!strcmp(id,"func"))     tt = TOK_DEF;
        else if (!strcmp(id,"println"))  tt = TOK_PRINT;
        else if (!strcmp(id,"let"))      tt = TOK_LET;
        else if (!strcmp(id,"const"))    tt = TOK_CONST;
        else if (!strcmp(id,"var"))      tt = TOK_VAR;
        else if (!strcmp(id,"new"))      tt = TOK_NEW;
        else if (!strcmp(id,"readln"))   tt = TOK_READLN;
        else if (!strcmp(id,"readi"))    tt = TOK_READI;
        else if (!strcmp(id,"readf"))    tt = TOK_READF;
        else if (!strcmp(id,"return"))   tt = TOK_RETURN;
        else if (!strcmp(id,"if"))       tt = TOK_IF;
        else if (!strcmp(id,"else"))     tt = TOK_ELSE;
        else if (!strcmp(id,"while"))    tt = TOK_WHILE;
        else if (!strcmp(id,"for"))      tt = TOK_FOR;
        else if (!strcmp(id,"in"))       tt = TOK_IN;
        else if (!strcmp(id,"range"))    tt = TOK_RANGE;
        else if (!strcmp(id,"package"))  tt = TOK_PACKAGE;
        else if (!strcmp(id,"pass"))     { free(id); return lexer_next(l); } /* skip pass */

        Token t2; t2.type = tt; t2.value = id;
        return t2;
    }

    /* numbers */
    if (isdigit((unsigned char)c)) {
        int start = l->pos;
        while (l->pos < l->len && isdigit((unsigned char)lexer_cur(l)))
            lexer_advance(l);
        return make_tok(TOK_NUMBER, strdup_n(l->src + start, l->pos - start));
    }

    /* string literals " or ' */
    if (c == '"' || c == '\'') {
        char quote = c;
        lexer_advance(l);
        int start = l->pos;
        while (l->pos < l->len && lexer_cur(l) != quote) {
            if (lexer_cur(l) == '\\') lexer_advance(l); /* skip escape */
            lexer_advance(l);
        }
        char *s = strdup_n(l->src + start, l->pos - start);
        if (l->pos < l->len) lexer_advance(l); /* consume closing quote */
        Token t2; t2.type = TOK_STRING_LITERAL; t2.value = s;
        return t2;
    }

    /* backtick strings */
    if (c == '`') {
        lexer_advance(l);
        int start = l->pos;
        while (l->pos < l->len && lexer_cur(l) != '`') lexer_advance(l);
        char *s = strdup_n(l->src + start, l->pos - start);
        if (l->pos < l->len) lexer_advance(l);
        Token t2; t2.type = TOK_BACKTICK_STRING; t2.value = s;
        return t2;
    }

    /* operators and punctuation */
    lexer_advance(l);
    switch (c) {
        case '+': return make_tok(TOK_PLUS, "+");
        case '-': return make_tok(TOK_MINUS, "-");
        case '*': return make_tok(TOK_MULTIPLY, "*");
        case '/': return make_tok(TOK_DIVIDE, "/");
        case '%': return make_tok(TOK_MODULO, "%");
        case '(': return make_tok(TOK_LPAREN, "(");
        case ')': return make_tok(TOK_RPAREN, ")");
	case '[': return make_tok(TOK_LBRACK, "[");
	case ']': return make_tok(TOK_RBRACK, "]");
        case '{': return make_tok(TOK_LBRACE, "{");
        case '}': return make_tok(TOK_RBRACE, "}");
        case ';': return make_tok(TOK_SEMICOLON, ";");
        case ',': return make_tok(TOK_COMMA, ",");
        case '.': return make_tok(TOK_DOT, ".");
        case ':': return make_tok(TOK_COLON, ":");
        case '@': return make_tok(TOK_AT, "@");
        case '=':
            if (lexer_cur(l) == '=') { lexer_advance(l); return make_tok(TOK_EQUAL_EQUAL, "=="); }
            return make_tok(TOK_EQUAL, "=");
        case '!':
            if (lexer_cur(l) == '=') { lexer_advance(l); return make_tok(TOK_NOT_EQUAL, "!="); }
            fprintf(stderr, "Lexer Error: unexpected '!'\n"); exit(1);
        case '<':
            if (lexer_cur(l) == '=') { lexer_advance(l); return make_tok(TOK_LESS_EQUAL, "<="); }
            return make_tok(TOK_LESS_THAN, "<");
        case '>':
            if (lexer_cur(l) == '=') { lexer_advance(l); return make_tok(TOK_GREATER_EQUAL, ">="); }
            return make_tok(TOK_GREATER_THAN, ">");
        default:
            fprintf(stderr, "Lexer Error: Invalid character '%c'\n", c);
            exit(1);
    }
}

/* Token array (dynamic) */
typedef struct {
    Token *tokens;
    int count;
    int cap;
} TokenList;

static void tl_init(TokenList *tl) { tl->tokens = NULL; tl->count = 0; tl->cap = 0; }
static void tl_push(TokenList *tl, Token t) {
    if (tl->count >= tl->cap) {
        tl->cap = tl->cap ? tl->cap * 2 : 64;
        tl->tokens = realloc(tl->tokens, tl->cap * sizeof(Token));
    }
    tl->tokens[tl->count++] = t;
}

static TokenList tokenize(const char *src) {
    char *clean = remove_comments(src);
    Lexer l; l.src = clean; l.pos = 0; l.len = strlen(clean);
    TokenList tl; tl_init(&tl);
    while (1) {
        Token t = lexer_next(&l);
        tl_push(&tl, t);
        if (t.type == TOK_EOF) break;
    }
    free(clean);
    return tl;
}

/* ============================================================
 * AST NODES
 * ============================================================ */

typedef enum {
    NODE_MAIN_DEF, NODE_FUNC_DEF, NODE_CLASS_DEF,
    NODE_VAR_DECL, NODE_ASSIGN, NODE_FIELD_DECL,
    NODE_PRINT, NODE_RETURN,
    NODE_IF, NODE_WHILE,
    NODE_FUNC_CALL, NODE_MEMBER_ACCESS, NODE_MEMBER_ASSIGN,
    NODE_NEW_EXPR, NODE_OBJECT_CREATION,
    NODE_BINARY_OP, NODE_UNARY_OP, NODE_LIST, NODE_INDEX, NODE_INDEX_ASSIGN,
    NODE_LITERAL_INT, NODE_LITERAL_STR, NODE_TEMPLATE_STR,
    NODE_VAR_ACCESS, NODE_READLN, NODE_READI, NODE_READF, NODE_FOR
} NodeType;

/* Dynamic array of AstNode* */
typedef struct {
    AstNode **items;
    int count;
    int cap;
} NodeList;

static void nl_init(NodeList *nl) { nl->items = NULL; nl->count = 0; nl->cap = 0; }
static void nl_push(NodeList *nl, AstNode *n) {
    if (nl->count >= nl->cap) {
        nl->cap = nl->cap ? nl->cap * 2 : 8;
        nl->items = realloc(nl->items, nl->cap * sizeof(AstNode*));
    }
    nl->items[nl->count++] = n;
}

/* Dynamic string list (for parameter names, etc.) */
typedef struct {
    char **items;
    int count;
    int cap;
} StrList;

static void sl_init(StrList *sl) { sl->items = NULL; sl->count = 0; sl->cap = 0; }
static void sl_push(StrList *sl, const char *s) {
    if (sl->count >= sl->cap) {
        sl->cap = sl->cap ? sl->cap * 2 : 4;
        sl->items = realloc(sl->items, sl->cap * sizeof(char*));
    }
    sl->items[sl->count++] = strdup(s);
}

struct AstNode {
    NodeType type;

    /* NODE_LITERAL_INT */
    long long int_val;

    /* NODE_LITERAL_STR, NODE_VAR_ACCESS, binary op operator, etc. */
    char *str_val;

    /* child nodes */
    AstNode *left;   /* binary_op left, unary operand, etc. */
    AstNode *right;  /* binary_op right */
    AstNode *expr;   /* print/return expression, condition */
    AstNode *object; /* member access object */

    /* lists */
    NodeList body;   /* main body, function body, if-then, etc. */
    NodeList args;   /* function call arguments */
    NodeList else_if_conds;  /* for if statement */
    NodeList else_if_bodies_flat; /* not ideal but simple */
    NodeList else_body;
    NodeList fields; /* class fields */
    NodeList methods; /* class methods */
    NodeList template_parts; /* template string parts */

    StrList params;  /* function parameters */
    StrList else_if_param; /* placeholder, not used */

    /* flags */
    int is_method;
    int is_const;   /* for var decl kind */
    int is_call;    /* member access: is it a call? */

    /* name (class name, function name, variable name, member name, etc.) */
    char *name;

    /* constructor node (for class defs) */
    AstNode *constructor;

    /* else-if blocks stored as parallel arrays */
    /* else_if_conds[i] = condition, else_if_bodies[i] is a NodeList */
    NodeList *else_if_bodies; /* array of NodeList, count = else_if_conds.count */
    int else_if_count;
};

static AstNode *new_node(NodeType t) {
    AstNode *n = calloc(1, sizeof(AstNode));
    n->type = t;
    nl_init(&n->body);
    nl_init(&n->args);
    nl_init(&n->else_if_conds);
    nl_init(&n->else_body);
    nl_init(&n->fields);
    nl_init(&n->methods);
    nl_init(&n->template_parts);
    sl_init(&n->params);
    return n;
}

/* ============================================================
 * PARSER
 * ============================================================ */

typedef struct {
    TokenList *tl;
    int idx;
} Parser;

static Token *p_cur(Parser *p) {
    if (p->idx < p->tl->count) return &p->tl->tokens[p->idx];
    return &p->tl->tokens[p->tl->count - 1]; /* EOF */
}
static Token *p_peek(Parser *p, int offset) {
    int i = p->idx + offset;
    if (i < p->tl->count) return &p->tl->tokens[i];
    return &p->tl->tokens[p->tl->count - 1];
}
static void p_advance(Parser *p) { p->idx++; }
static void p_eat(Parser *p, TokenType expected) {
    if (p_cur(p)->type == expected) { p_advance(p); return; }
    fprintf(stderr, "Syntax Error: expected token type %d but got %d ('%s') at index %d\n",
        expected, p_cur(p)->type, p_cur(p)->value, p->idx);
    exit(1);
}

/* Forward declarations */
static AstNode *parse_expression(Parser *p);
static NodeList parse_body(Parser *p);
static AstNode *parse_statement(Parser *p);
static AstNode *parse_function_definition(Parser *p, int is_method);
static AstNode *parse_class_definition(Parser *p);

static AstNode *parse_primary(Parser *p) {
    Token *cur = p_cur(p);

    if (cur->type == TOK_NUMBER) {
        AstNode *n = new_node(NODE_LITERAL_INT);
        n->int_val = atoll(cur->value);
        p_advance(p);
        return n;
    }
    if (cur->type == TOK_STRING_LITERAL) {
        AstNode *n = new_node(NODE_LITERAL_STR);
        n->str_val = strdup(cur->value);
        p_advance(p);
        return n;
    }
    if (cur->type == TOK_BACKTICK_STRING) {
        /* Parse template string: split on ${varname} */
        AstNode *n = new_node(NODE_TEMPLATE_STR);
        char *s = cur->value;
        p_advance(p);
        /* Simple parser: find ${...} */
        int i = 0, len = strlen(s);
        char buf[4096];
        int buf_len = 0;
        while (i <= len) {
            if (i < len && s[i] == '$' && s[i+1] == '{') {
                /* flush literal */
                if (buf_len > 0) {
                    buf[buf_len] = '\0';
                    AstNode *lit = new_node(NODE_LITERAL_STR);
                    lit->str_val = strdup(buf);
                    nl_push(&n->template_parts, lit);
                    buf_len = 0;
                }
                i += 2; /* skip ${ */
                int vstart = i;
                while (i < len && s[i] != '}') i++;
                char varexpr[256];
                int vlen = i - vstart;
                memcpy(varexpr, s + vstart, vlen);
                varexpr[vlen] = '\0';
                i++; /* skip } */

                /* Parse varexpr: may be "name" or "obj.field" */
                AstNode *part_node;
                char *dot = strchr(varexpr, '.');
                if (dot) {
                    *dot = '\0';
                    char *obj_n = varexpr, *mem_n = dot + 1;
                    AstNode *obj_node = new_node(NODE_VAR_ACCESS);
                    obj_node->name = strdup(obj_n);
                    part_node = new_node(NODE_MEMBER_ACCESS);
                    part_node->object = obj_node;
                    part_node->name = strdup(mem_n);
                    part_node->is_call = 0;
                } else {
                    part_node = new_node(NODE_VAR_ACCESS);
                    part_node->name = strdup(varexpr);
                }
                nl_push(&n->template_parts, part_node);
            } else {
                if (i < len) buf[buf_len++] = s[i++];
                else {
                    if (buf_len > 0) {
                        buf[buf_len] = '\0';
                        AstNode *lit = new_node(NODE_LITERAL_STR);
                        lit->str_val = strdup(buf);
                        nl_push(&n->template_parts, lit);
                    }
                    break;
                }
            }
        }
        return n;
    }
    if (cur->type == TOK_READLN) {
        p_advance(p); /* consume readln */
        p_eat(p, TOK_LPAREN);
        /* prompt expression */
        AstNode *prompt_node = parse_expression(p);
        p_eat(p, TOK_RPAREN);
        AstNode *n = new_node(NODE_READLN);
        n->expr = prompt_node;
        return n;
    }
    if (cur->type == TOK_READI){
    	p_advance(p);
        p_eat(p,TOK_LPAREN);
        AstNode *prompt_node= parse_expression(p);
        p_eat(p,TOK_RPAREN);
        AstNode *n=new_node(NODE_READI);
        n->expr =prompt_node;
        return n;
    }
    if (cur->type==TOK_READF){
       p_advance(p);
       p_eat(p,TOK_LPAREN);
       AstNode *prompt_node=parse_expression(p);
       p_eat(p,TOK_RPAREN);
       AstNode *n=new_node(NODE_READF);
       n->expr=prompt_node;
       return n;
    }
    if (cur->type == TOK_LBRACK) {
        p_advance(p); /* consume [ */
        AstNode *n = new_node(NODE_LIST);
        if (p_cur(p)->type != TOK_RBRACK) {
            nl_push(&n->args, parse_expression(p));
            while (p_cur(p)->type == TOK_COMMA) {
                p_advance(p);
                if (p_cur(p)->type == TOK_RBRACK) break; /* trailing comma */
                nl_push(&n->args, parse_expression(p));
            }
        }
        p_eat(p, TOK_RBRACK);
        return n;
    }
    if (cur->type == TOK_NEW) {
        p_advance(p);
        char *class_name = strdup(p_cur(p)->value);
        p_eat(p, TOK_IDENTIFIER);
        p_eat(p, TOK_LPAREN);
        AstNode *n = new_node(NODE_OBJECT_CREATION);
        n->name = class_name;
        if (p_cur(p)->type != TOK_RPAREN) {
            nl_push(&n->args, parse_expression(p));
            while (p_cur(p)->type == TOK_COMMA) {
                p_advance(p);
                nl_push(&n->args, parse_expression(p));
            }
        }
        p_eat(p, TOK_RPAREN);
        return n;
    }
    if (cur->type == TOK_LPAREN) {
        p_advance(p);
        AstNode *e = parse_expression(p);
        p_eat(p, TOK_RPAREN);
        return e;
    }
    if (cur->type == TOK_SELF) {
        p_advance(p);
        AstNode *n = new_node(NODE_VAR_ACCESS);
        n->name = strdup("self");
        if (p_cur(p)->type == TOK_DOT) {
            while (p_cur(p)->type == TOK_DOT) {
                p_advance(p);
                char *member = strdup(p_cur(p)->value);
                p_eat(p, TOK_IDENTIFIER);
                AstNode *ma = new_node(NODE_MEMBER_ACCESS);
                ma->object = n;
                ma->name = member;
                if (p_cur(p)->type == TOK_LPAREN) {
                    ma->is_call = 1;
                    p_advance(p);
                    if (p_cur(p)->type != TOK_RPAREN) {
                        nl_push(&ma->args, parse_expression(p));
                        while (p_cur(p)->type == TOK_COMMA) {
                            p_advance(p);
                            nl_push(&ma->args, parse_expression(p));
                        }
                    }
                    p_eat(p, TOK_RPAREN);
                }
                n = ma;
            }
        }
        return n;
    }
    if (cur->type == TOK_IDENTIFIER) {
        char *name = strdup(cur->value);
        p_advance(p);
        Token *nxt = p_cur(p);
        if (nxt->type == TOK_LPAREN) {
            /* function call */
            p_advance(p);
            AstNode *n = new_node(NODE_FUNC_CALL);
            n->name = name;
            if (p_cur(p)->type != TOK_RPAREN) {
                nl_push(&n->args, parse_expression(p));
                while (p_cur(p)->type == TOK_COMMA) {
                    p_advance(p);
                    nl_push(&n->args, parse_expression(p));
                }
            }
            p_eat(p, TOK_RPAREN);
            return n;
        }
        if (nxt->type == TOK_DOT) {
            /* member access chain */
            AstNode *obj = new_node(NODE_VAR_ACCESS);
            obj->name = name;
            while (p_cur(p)->type == TOK_DOT) {
                p_advance(p); /* eat . */
                char *member = strdup(p_cur(p)->value);
                p_eat(p, TOK_IDENTIFIER);
                AstNode *ma = new_node(NODE_MEMBER_ACCESS);
                ma->object = obj;
                ma->name = member;
                if (p_cur(p)->type == TOK_LPAREN) {
                    ma->is_call = 1;
                    p_advance(p);
                    if (p_cur(p)->type != TOK_RPAREN) {
                        nl_push(&ma->args, parse_expression(p));
                        while (p_cur(p)->type == TOK_COMMA) {
                            p_advance(p);
                            nl_push(&ma->args, parse_expression(p));
                        }
                    }
                    p_eat(p, TOK_RPAREN);
                }
                obj = ma;
            }
            return obj;
        }
        /* plain variable access */
        AstNode *n = new_node(NODE_VAR_ACCESS);
        n->name = name;
        return n;
    }

    fprintf(stderr, "Syntax Error: unexpected token '%s' (type %d) in primary expression\n",
        cur->value, cur->type);
    exit(1);
}


static AstNode *parse_postfix(Parser *p) {
    AstNode *node = parse_primary(p);
    /* handle postfix index access: expr[index] */
    while (p_cur(p)->type == TOK_LBRACK) {
        p_advance(p); /* consume [ */
        AstNode *idx = parse_expression(p);
        p_eat(p, TOK_RBRACK);
        AstNode *n = new_node(NODE_INDEX);
        n->left = node;
        n->right = idx;
        node = n;
    }
    return node;
}
static AstNode *parse_factor(Parser *p) {
    /* unary +/- */
    if (p_cur(p)->type == TOK_PLUS || p_cur(p)->type == TOK_MINUS) {
        char *op = strdup(p_cur(p)->value);
        p_advance(p);
        AstNode *n = new_node(NODE_UNARY_OP);
        n->str_val = op;
        n->left = parse_postfix(p);
        return n;
    }
    AstNode *left = parse_postfix(p);
    while (p_cur(p)->type == TOK_MULTIPLY ||
           p_cur(p)->type == TOK_DIVIDE ||
           p_cur(p)->type == TOK_MODULO) {
        char *op = strdup(p_cur(p)->value);
        p_advance(p);
        AstNode *n = new_node(NODE_BINARY_OP);
        n->str_val = op;
        n->left = left;
        n->right = parse_primary(p);
        left = n;
    }
    return left;
}

static AstNode *parse_term(Parser *p) {
    AstNode *left = parse_factor(p);
    while (p_cur(p)->type == TOK_PLUS || p_cur(p)->type == TOK_MINUS) {
        char *op = strdup(p_cur(p)->value);
        p_advance(p);
        AstNode *n = new_node(NODE_BINARY_OP);
        n->str_val = op;
        n->left = left;
        n->right = parse_factor(p);
        left = n;
    }
    return left;
}

static AstNode *parse_expression(Parser *p) {
    AstNode *left = parse_term(p);
    while (p_cur(p)->type == TOK_EQUAL_EQUAL ||
           p_cur(p)->type == TOK_NOT_EQUAL ||
           p_cur(p)->type == TOK_LESS_THAN ||
           p_cur(p)->type == TOK_GREATER_THAN ||
           p_cur(p)->type == TOK_LESS_EQUAL ||
           p_cur(p)->type == TOK_GREATER_EQUAL) {
        char *op = strdup(p_cur(p)->value);
        p_advance(p);
        AstNode *n = new_node(NODE_BINARY_OP);
        n->str_val = op;
        n->left = left;
        n->right = parse_term(p);
        left = n;
    }
    return left;
}

static AstNode *parse_var_decl(Parser *p) {
    int is_const = (p_cur(p)->type == TOK_CONST);
    p_advance(p); /* consume let/const/var */
    char *name = strdup(p_cur(p)->value);
    p_eat(p, TOK_IDENTIFIER);
    p_eat(p, TOK_EQUAL);
    AstNode *val = parse_expression(p);
    p_eat(p, TOK_SEMICOLON);
    AstNode *n = new_node(NODE_VAR_DECL);
    n->name = name;
    n->is_const = is_const;
    n->expr = val;
    return n;
}

static AstNode *parse_assign(Parser *p) {
    char *name = strdup(p_cur(p)->value);
    p_eat(p, TOK_IDENTIFIER);
    p_eat(p, TOK_EQUAL);
    AstNode *val = parse_expression(p);
    p_eat(p, TOK_SEMICOLON);
    AstNode *n = new_node(NODE_ASSIGN);
    n->name = name;
    n->expr = val;
    return n;
}

static AstNode *parse_print(Parser *p) {
    p_eat(p, TOK_PRINT);
    p_eat(p, TOK_LPAREN);
    AstNode *e = parse_expression(p);
    p_eat(p, TOK_RPAREN);
    p_eat(p, TOK_SEMICOLON);
    AstNode *n = new_node(NODE_PRINT);
    n->expr = e;
    return n;
}

static AstNode *parse_return_stmt(Parser *p) {
    p_eat(p, TOK_RETURN);
    AstNode *e = parse_expression(p);
    p_eat(p, TOK_SEMICOLON);
    AstNode *n = new_node(NODE_RETURN);
    n->expr = e;
    return n;
}

static AstNode *parse_if(Parser *p) {
    p_eat(p, TOK_IF);
    p_eat(p, TOK_LPAREN);
    AstNode *cond = parse_expression(p);
    p_eat(p, TOK_RPAREN);
    p_eat(p, TOK_LBRACE);
    NodeList then_body = parse_body(p);
    p_eat(p, TOK_RBRACE);

    AstNode *n = new_node(NODE_IF);
    n->expr = cond;
    n->body = then_body;

    int ei_cap = 0;
    n->else_if_count = 0;
    n->else_if_bodies = NULL;

    while (p_cur(p)->type == TOK_ELSE) {
        p_advance(p);
        if (p_cur(p)->type == TOK_IF) {
            p_advance(p);
            p_eat(p, TOK_LPAREN);
            AstNode *ei_cond = parse_expression(p);
            p_eat(p, TOK_RPAREN);
            p_eat(p, TOK_LBRACE);
            NodeList ei_body = parse_body(p);
            p_eat(p, TOK_RBRACE);
            nl_push(&n->else_if_conds, ei_cond);
            if (n->else_if_count >= ei_cap) {
                ei_cap = ei_cap ? ei_cap * 2 : 4;
                n->else_if_bodies = realloc(n->else_if_bodies, ei_cap * sizeof(NodeList));
            }
            n->else_if_bodies[n->else_if_count++] = ei_body;
        } else {
            p_eat(p, TOK_LBRACE);
            n->else_body = parse_body(p);
            p_eat(p, TOK_RBRACE);
            break;
        }
    }
    return n;
}

static AstNode *parse_while(Parser *p) {
    p_eat(p, TOK_WHILE);
    p_eat(p, TOK_LPAREN);
    AstNode *cond = parse_expression(p);
    p_eat(p, TOK_RPAREN);
    p_eat(p, TOK_LBRACE);
    NodeList body = parse_body(p);
    p_eat(p, TOK_RBRACE);
    AstNode *n = new_node(NODE_WHILE);
    n->expr = cond;
    n->body = body;
    return n;
}

/* Parse the post-step of a C-style for loop (no trailing semicolon).
   Supports: i++, i--, i=expr */
static AstNode *parse_for_post(Parser *p) {
    if (p_cur(p)->type == TOK_IDENTIFIER) {
        char *name = strdup(p_cur(p)->value);
        p_advance(p); /* consume identifier, now p_cur() is the operator */
        /* i++ : check current is PLUS, next is also PLUS */
        if (p_cur(p)->type == TOK_PLUS && p_peek(p, 1)->type == TOK_PLUS) {
            p_advance(p); p_advance(p); /* consume both + */
            AstNode *one = new_node(NODE_LITERAL_INT); one->int_val = 1;
            AstNode *var = new_node(NODE_VAR_ACCESS); var->name = strdup(name);
            AstNode *add = new_node(NODE_BINARY_OP); add->str_val = strdup("+");
            add->left = var; add->right = one;
            AstNode *n = new_node(NODE_ASSIGN); n->name = name; n->expr = add;
            free(name);
            return n;
        }
        /* i-- */
        if (p_cur(p)->type == TOK_MINUS && p_peek(p, 1)->type == TOK_MINUS) {
            p_advance(p); p_advance(p); /* consume both - */
            AstNode *one = new_node(NODE_LITERAL_INT); one->int_val = 1;
            AstNode *var = new_node(NODE_VAR_ACCESS); var->name = strdup(name);
            AstNode *sub = new_node(NODE_BINARY_OP); sub->str_val = strdup("-");
            sub->left = var; sub->right = one;
            AstNode *n = new_node(NODE_ASSIGN); n->name = name; n->expr = sub;
            free(name);
            return n;
        }
        /* i = expr */
        if (p_cur(p)->type == TOK_EQUAL) {
            p_advance(p);
            AstNode *val = parse_expression(p);
            AstNode *n = new_node(NODE_ASSIGN); n->name = name; n->expr = val;
            free(name);
            return n;
        }
        free(name);
    }
    fprintf(stderr, "Syntax Error: invalid for-loop post-step\n"); exit(1);
}

static AstNode *parse_for(Parser *p) {
    p_eat(p, TOK_FOR);
    p_eat(p, TOK_LPAREN);

    /* Peek ahead to distinguish C-style vs for-in:
       C-style:  for (let i=0; cond; post) {}
       For-in:   for (let e:TypeHint in list) {} */

    /* Both start with let/const/var or an identifier declaration */
    /* We detect by looking for ':' or 'in' after the var name */

    AstNode *n = new_node(NODE_FOR);

    /* consume the variable declaration: let/const/var name */
    if (p_cur(p)->type != TOK_LET && p_cur(p)->type != TOK_CONST && p_cur(p)->type != TOK_VAR) {
        fprintf(stderr, "Syntax Error: for loop expects variable declaration\n"); exit(1);
    }
    p_advance(p); /* consume let/const/var */
    char *var_name = strdup(p_cur(p)->value);
    p_eat(p, TOK_IDENTIFIER);
    n->name = var_name;

    /* check for for-in: next token is ':' (type hint) or 'in' */
    if (p_cur(p)->type == TOK_COLON || p_cur(p)->type == TOK_IN) {
        /* for-in: for (let e:TypeHint in list) {} */
        n->is_method = 1; /* flag: this is a for-in loop */
        if (p_cur(p)->type == TOK_COLON) {
            p_eat(p,TOK_COLON); /* consume : */
            char* type=p_cur(p)->value;
            p_advance(p); /* consume type hint identifier (ignored) */
        }
        p_eat(p, TOK_IN);
        n->object = parse_expression(p); /* iterable */
        p_eat(p, TOK_RPAREN);
        p_eat(p, TOK_LBRACE);
        n->body = parse_body(p);
        p_eat(p, TOK_RBRACE);
    } else {
        /* C-style: for (let i=0; cond; post) {} */
        n->is_method = 0;
        p_eat(p, TOK_EQUAL);
        n->left = parse_expression(p); /* init value */
        p_eat(p, TOK_SEMICOLON);
        n->expr = parse_expression(p); /* condition */
        p_eat(p, TOK_SEMICOLON);
        n->right = parse_for_post(p);  /* post step */
        p_eat(p, TOK_RPAREN);
        p_eat(p, TOK_LBRACE);
        n->body = parse_body(p);
        p_eat(p, TOK_RBRACE);
    }
    return n;
}
static AstNode *parse_statement(Parser *p) {
    Token *cur = p_cur(p);

    if (cur->type == TOK_PRINT) return parse_print(p);
    if (cur->type == TOK_LET || cur->type == TOK_CONST || cur->type == TOK_VAR)
        return parse_var_decl(p);
    if (cur->type == TOK_DEF) return parse_function_definition(p, 0);
    if (cur->type == TOK_PUBLIC) {
        p_advance(p); /* eat public */
        if (p_cur(p)->type == TOK_CLASS) return parse_class_definition(p);
        fprintf(stderr, "Syntax Error: expected 'class' after 'public'\n"); exit(1);
    }
    if (cur->type == TOK_CLASS) return parse_class_definition(p);
    if (cur->type == TOK_IF) return parse_if(p);
    if (cur->type == TOK_WHILE) return parse_while(p);
    if (cur->type == TOK_FOR) return parse_for(p);
    if (cur->type == TOK_RETURN) return parse_return_stmt(p);

    /* index assignment: name[expr] = expr; */
    if (cur->type == TOK_IDENTIFIER && p_peek(p, 1)->type == TOK_LBRACK) {
        AstNode *obj = new_node(NODE_VAR_ACCESS);
        obj->name = strdup(cur->value);
        p_advance(p); /* eat identifier */
        p_advance(p); /* eat [ */
        AstNode *idx = parse_expression(p);
        p_eat(p, TOK_RBRACK);
        p_eat(p, TOK_EQUAL);
        AstNode *val = parse_expression(p);
        p_eat(p, TOK_SEMICOLON);
        AstNode *n = new_node(NODE_INDEX_ASSIGN);
        n->object = obj;
        n->left = idx;
        n->expr = val;
        return n;
    }
    if (cur->type == TOK_IDENTIFIER || cur->type == TOK_SELF) {
        Token *nxt = p_peek(p, 1);
        if (cur->type == TOK_IDENTIFIER && nxt->type == TOK_EQUAL) return parse_assign(p);
        if (nxt->type == TOK_DOT) {
            /* member assign or member call */
            char *obj_name = strdup(cur->value);
            p_advance(p); /* eat identifier or self */
            AstNode *obj = new_node(NODE_VAR_ACCESS);
            obj->name = obj_name;

            /* follow dot chain */
            while (p_cur(p)->type == TOK_DOT) {
                p_advance(p);
                char *member = strdup(p_cur(p)->value);
                p_eat(p, TOK_IDENTIFIER);
                if (p_cur(p)->type == TOK_EQUAL) {
                    /* member assignment */
                    p_advance(p);
                    AstNode *val = parse_expression(p);
                    p_eat(p, TOK_SEMICOLON);
                    AstNode *n = new_node(NODE_MEMBER_ASSIGN);
                    n->object = obj;
                    n->name = member;
                    n->expr = val;
                    return n;
                }
                AstNode *ma = new_node(NODE_MEMBER_ACCESS);
                ma->object = obj;
                ma->name = member;
                if (p_cur(p)->type == TOK_LPAREN) {
                    ma->is_call = 1;
                    p_advance(p);
                    if (p_cur(p)->type != TOK_RPAREN) {
                        nl_push(&ma->args, parse_expression(p));
                        while (p_cur(p)->type == TOK_COMMA) {
                            p_advance(p);
                            nl_push(&ma->args, parse_expression(p));
                        }
                    }
                    p_eat(p, TOK_RPAREN);
                }
                obj = ma;
            }
            /* If it ended as a call, eat semicolon */
            if (obj->type == NODE_MEMBER_ACCESS && obj->is_call) {
                p_eat(p, TOK_SEMICOLON);
            }
            return obj;
        }
        if (cur->type == TOK_IDENTIFIER && nxt->type == TOK_LPAREN) {
            /* function call statement */
            AstNode *e = parse_primary(p);
            p_eat(p, TOK_SEMICOLON);
            return e;
        }
    }

    fprintf(stderr, "Syntax Error: unexpected token '%s' (type %d) in statement\n",
        cur->value, cur->type);
    exit(1);
}

static NodeList parse_body(Parser *p) {
    NodeList nl; nl_init(&nl);
    while (p_cur(p)->type != TOK_RBRACE && p_cur(p)->type != TOK_EOF) {
        nl_push(&nl, parse_statement(p));
    }
    return nl;
}

static AstNode *parse_function_definition(Parser *p, int is_method) {
    p_eat(p, TOK_DEF);
    char *name = strdup(p_cur(p)->value);
    p_eat(p, TOK_IDENTIFIER);
    p_eat(p, TOK_LPAREN);
    AstNode *n = new_node(NODE_FUNC_DEF);
    n->name = name;
    n->is_method = is_method;

    /* parameters */
    if (p_cur(p)->type != TOK_RPAREN) {
        if (p_cur(p)->type == TOK_SELF) {
            sl_push(&n->params, "self");
            p_advance(p);
        } else {
            sl_push(&n->params, p_cur(p)->value);
            p_eat(p, TOK_IDENTIFIER);
        }
        while (p_cur(p)->type == TOK_COMMA) {
            p_advance(p);
            sl_push(&n->params, p_cur(p)->value);
            p_eat(p, TOK_IDENTIFIER);
        }
    }
    p_eat(p, TOK_RPAREN);
    p_eat(p, TOK_LBRACE);
    n->body = parse_body(p);
    p_eat(p, TOK_RBRACE);
    return n;
}

static AstNode *parse_class_definition(Parser *p) {
    p_eat(p, TOK_CLASS);
    char *name = strdup(p_cur(p)->value);
    p_eat(p, TOK_IDENTIFIER);
    p_eat(p, TOK_LPAREN);
    p_eat(p, TOK_AT);
    p_eat(p, TOK_INNERSELF);
    p_eat(p, TOK_RPAREN);
    p_eat(p, TOK_LBRACE);

    AstNode *n = new_node(NODE_CLASS_DEF);
    n->name = name;
    n->constructor = NULL;

    while (p_cur(p)->type != TOK_RBRACE && p_cur(p)->type != TOK_EOF) {
        if (p_cur(p)->type == TOK_LET || p_cur(p)->type == TOK_CONST) {
            int is_const = (p_cur(p)->type == TOK_CONST);
            p_advance(p);
            char *fname = strdup(p_cur(p)->value);
            p_eat(p, TOK_IDENTIFIER);
            AstNode *fval = NULL;
            if (p_cur(p)->type == TOK_EQUAL) { p_advance(p); fval = parse_expression(p); }
            p_eat(p, TOK_SEMICOLON);
            AstNode *fd = new_node(NODE_FIELD_DECL);
            fd->name = fname;
            fd->is_const = is_const;
            fd->expr = fval;
            nl_push(&n->fields, fd);
        } else if (p_cur(p)->type == TOK_DEF) {
            AstNode *method = parse_function_definition(p, 1);
            if (!strcmp(method->name, "init")) n->constructor = method;
            else nl_push(&n->methods, method);
        } else {
            fprintf(stderr, "Syntax Error: unexpected token in class body: '%s'\n", p_cur(p)->value);
            exit(1);
        }
    }
    p_eat(p, TOK_RBRACE);
    return n;
}

static AstNode *parse_program(Parser *p) {
    p_eat(p, TOK_PUBLIC);
    p_eat(p, TOK_CLASS);
    p_eat(p, TOK_MAIN);
    p_eat(p, TOK_LPAREN);
    p_eat(p, TOK_AT);
    p_eat(p, TOK_SELF);
    p_eat(p, TOK_RPAREN);
    p_eat(p, TOK_LBRACE);
    AstNode *prog = new_node(NODE_MAIN_DEF);
    prog->body = parse_body(p);
    p_eat(p, TOK_RBRACE);
    return prog;
}

/* ============================================================
 * RUNTIME VALUES
 * ============================================================ */

typedef enum {
    VAL_INT, VAL_DOUBLE, VAL_STR, VAL_BOOL, VAL_NULL, VAL_OBJECT, VAL_LIST
} ValueType;

typedef struct PJField {
    char *name;
    Value *val;
    int is_const;
} PJField;

struct PJObject {
    char *class_name;
    PJField *fields;
    int field_count;
    int field_cap;
    /* methods are looked up from class definition */
    /* we store pointer to class def */
    AstNode *class_def; /* NODE_CLASS_DEF */
};

typedef struct PJList {
    Value **items;
    int count;
    int cap;
} PJList;

static PJList *pjlist_new(void) {
    PJList *l = calloc(1, sizeof(PJList));
    return l;
}
static void pjlist_push(PJList *l, Value *v) {
    if (l->count >= l->cap) {
        l->cap = l->cap ? l->cap * 2 : 4;
        l->items = realloc(l->items, l->cap * sizeof(Value*));
    }
    l->items[l->count++] = v;
}

struct Value {
    ValueType type;
    long long ival;
    long double fval;
    char *sval;
    PJObject *obj;
    PJList *list;
    int ref_count; /* simple ref counting for memory management */
};
long long longround(long double val){
    return (long long)val;
}
static Value *val_int(long long v) {
    Value *val = malloc(sizeof(Value));
    val->type = VAL_INT; val->ival = v; val->fval=v; val->sval = NULL; val->obj = NULL; val->list = NULL; val->ref_count = 1;
    return val;
}
static Value *val_double(long double v){
    Value *val=malloc(sizeof(Value));
    val->type=VAL_DOUBLE; val->ival=longround(v); val->fval=v; val->sval=NULL; val->obj=NULL; val->list=NULL; val->ref_count=1;
    return val;
}
static Value *val_str(const char *s) {
    Value *val = malloc(sizeof(Value));
    val->type = VAL_STR; val->sval = strdup(s); val->ival = 0; val->fval=0.0; val->obj = NULL; val->list = NULL; val->ref_count = 1;
    return val;
}
static Value *val_bool(int b) { return val_int(b ? 1 : 0); }
static Value *val_null(void) {
    Value *val = malloc(sizeof(Value));
    val->type = VAL_NULL; val->sval = NULL; val->ival = 0; val->obj = NULL; val->list = NULL; val->ref_count = 1;
    return val;
}
static Value *val_obj(PJObject *obj) {
    Value *val = malloc(sizeof(Value));
    val->type = VAL_OBJECT; val->obj = obj; val->sval = NULL; val->ival = 0; val->list = NULL; val->ref_count = 1;
    return val;
}
static Value *val_list(PJList *l) {
    Value *val = malloc(sizeof(Value));
    val->type = VAL_LIST; val->list = l; val->sval = NULL; val->ival = 0; val->obj = NULL; val->ref_count = 1;
    return val;
}

static void val_print(Value *v) {
    if (!v) { printf("null"); return; }
    switch (v->type) {
        case VAL_INT:    printf("%lld", v->ival); break;
        case VAL_DOUBLE: printf("%Lg", v->fval); break;
        case VAL_STR:    printf("%s", v->sval); break;
        case VAL_BOOL:   printf("%s", v->ival ? "true" : "false"); break;
        case VAL_NULL:   printf("null"); break;
        case VAL_OBJECT:
            printf("<%s object>", v->obj ? v->obj->class_name : "?");
            break;
        case VAL_LIST: {
            printf("[");
            for (int i = 0; i < v->list->count; i++) {
                if (i > 0) printf(", ");
                /* print strings with quotes inside lists */
                if (v->list->items[i]->type == VAL_STR)
                    printf("'%s'", v->list->items[i]->sval);
                else
                    val_print(v->list->items[i]);
            }
            printf("]");
            break;
        }
    }
}

static char *val_to_str(Value *v) {
    if (!v) return strdup("null");
    char buf[64];
    switch (v->type) {
        case VAL_INT:    sprintf(buf, "%lld", v->ival); return strdup(buf);
        case VAL_DOUBLE: sprintf(buf, "%Lg", v->fval); return strdup(buf);
        case VAL_STR:    return strdup(v->sval);
        case VAL_BOOL:   return strdup(v->ival ? "true" : "false");
        case VAL_NULL:   return strdup("null");
        case VAL_OBJECT: {
            char *s = malloc(strlen(v->obj->class_name) + 16);
            sprintf(s, "<%s object>", v->obj->class_name);
            return s;
        }
        case VAL_LIST: {
            /* build "[a, b, c]" string */
            char *res = strdup("[");
            for (int i = 0; i < v->list->count; i++) {
                if (i > 0) { char *t = malloc(strlen(res)+3); strcpy(t,res); strcat(t,", "); free(res); res=t; }
                char *item;
                if (v->list->items[i]->type == VAL_STR) {
                    item = malloc(strlen(v->list->items[i]->sval)+3);
                    sprintf(item, "'%s'", v->list->items[i]->sval);
                } else {
                    item = val_to_str(v->list->items[i]);
                }
                char *t = malloc(strlen(res)+strlen(item)+1);
                strcpy(t,res); strcat(t,item); free(res); free(item); res=t;
            }
            char *t = malloc(strlen(res)+2); strcpy(t,res); strcat(t,"]"); free(res);
            return t;
        }
    }
    return strdup("");
}

static int val_truthy(Value *v) {
    if (!v) return 0;
    if (v->type == VAL_NULL) return 0;
    if (v->type == VAL_INT || v->type == VAL_BOOL) return v->ival != 0;
    if (v->type == VAL_STR) return v->sval && v->sval[0] != '\0';
    return 1;
}

/* ============================================================
 * SCOPE / ENVIRONMENT
 * ============================================================ */

#define SCOPE_BUCKETS 64
typedef struct VarEntry {
    char *name;
    Value *val;
    int is_const;
    struct VarEntry *next;
} VarEntry;

struct ScopeTable {
    VarEntry *buckets[SCOPE_BUCKETS];
    ScopeTable *parent;
};

static unsigned int hash_str(const char *s) {
    unsigned int h = 5381;
    while (*s) h = h * 33 + (unsigned char)*s++;
    return h % SCOPE_BUCKETS;
}



static ScopeTable *scope_new(ScopeTable *parent) {
    ScopeTable *st = calloc(1, sizeof(ScopeTable));
    st->parent = parent;
    return st;
}

static void scope_set(ScopeTable *st, const char *name, Value *val, int is_const, int declare) {
    if (declare) {
        unsigned int h = hash_str(name);
        /* check not already declared in current scope */
        for (VarEntry *e = st->buckets[h]; e; e = e->next)
            if (!strcmp(e->name, name)) {
                fprintf(stderr, "Runtime Error: Redeclaration of '%s'\n", name); exit(1);
            }
        VarEntry *e = malloc(sizeof(VarEntry));
        e->name = strdup(name); e->val = val; e->is_const = is_const; e->next = st->buckets[h];
        st->buckets[h] = e;
        return;
    }
    /* assignment: find in scope chain */
    for (ScopeTable *s = st; s; s = s->parent) {
        unsigned int h = hash_str(name);
        for (VarEntry *e = s->buckets[h]; e; e = e->next)
            if (!strcmp(e->name, name)) {
                if (e->is_const) { fprintf(stderr, "Runtime Error: Cannot assign to const '%s'\n", name); exit(1); }
                e->val = val;
                return;
            }
    }
    fprintf(stderr, "Runtime Error: Assignment to undefined variable '%s'\n", name); exit(1);
}

static Value *scope_get(ScopeTable *st, const char *name) {
    for (ScopeTable *s = st; s; s = s->parent) {
        unsigned int h = hash_str(name);
        for (VarEntry *e = s->buckets[h]; e; e = e->next)
            if (!strcmp(e->name, name)) return e->val;
    }
    fprintf(stderr, "Runtime Error: Undefined variable '%s'\n", name); exit(1);
}

/* ============================================================
 * INTERPRETER
 * ============================================================ */

/* Global tables */
typedef struct {
    char *name;
    AstNode *def; /* NODE_FUNC_DEF or NODE_CLASS_DEF */
} GlobalEntry;

typedef struct {
    GlobalEntry *funcs;
    int func_count, func_cap;
    GlobalEntry *classes;
    int class_count, class_cap;
    ScopeTable *scope; /* current scope */
} Env;

static void env_reg_func(Env *env, const char *name, AstNode *def) {
    if (env->func_count >= env->func_cap) {
        env->func_cap = env->func_cap ? env->func_cap * 2 : 16;
        env->funcs = realloc(env->funcs, env->func_cap * sizeof(GlobalEntry));
    }
    env->funcs[env->func_count].name = strdup(name);
    env->funcs[env->func_count].def = def;
    env->func_count++;
}

static AstNode *env_find_func(Env *env, const char *name) {
    for (int i = 0; i < env->func_count; i++)
        if (!strcmp(env->funcs[i].name, name)) return env->funcs[i].def;
    return NULL;
}

static void env_reg_class(Env *env, const char *name, AstNode *def) {
    if (env->class_count >= env->class_cap) {
        env->class_cap = env->class_cap ? env->class_cap * 2 : 16;
        env->classes = realloc(env->classes, env->class_cap * sizeof(GlobalEntry));
    }
    env->classes[env->class_count].name = strdup(name);
    env->classes[env->class_count].def = def;
    env->class_count++;
}

static AstNode *env_find_class(Env *env, const char *name) {
    for (int i = 0; i < env->class_count; i++)
        if (!strcmp(env->classes[i].name, name)) return env->classes[i].def;
    return NULL;
}

/* Return value signaling via longjmp would be complex; use a simpler approach:
   store return value in env and use a flag */
static Value *g_return_val = NULL;
static int g_returning = 0;

static Value *visit(Env *env, AstNode *node, PJObject *self_obj);


/* interpreter helpers */

/* Object field helpers */
static PJField *obj_find_field(PJObject *obj, const char *name) {
    for (int i = 0; i < obj->field_count; i++)
        if (!strcmp(obj->fields[i].name, name)) return &obj->fields[i];
    return NULL;
}

static void obj_set_field(PJObject *obj, const char *name, Value *val) {
    for (int i = 0; i < obj->field_count; i++)
        if (!strcmp(obj->fields[i].name, name)) {
            if (obj->fields[i].is_const) {
                fprintf(stderr, "Runtime Error: Cannot assign to const field '%s'\n", name); exit(1);
            }
            obj->fields[i].val = val;
            return;
        }
    fprintf(stderr, "Runtime Error: Field '%s' not found on object '%s'\n", name, obj->class_name);
    exit(1);
}

static Value *obj_get_field(PJObject *obj, const char *name) {
    for (int i = 0; i < obj->field_count; i++)
        if (!strcmp(obj->fields[i].name, name)) return obj->fields[i].val;
    fprintf(stderr, "Runtime Error: Field '%s' not found on object '%s'\n", name, obj->class_name);
    exit(1);
}

static AstNode *obj_find_method(PJObject *obj, const char *name) {
    AstNode *cls = obj->class_def;
    for (int i = 0; i < cls->methods.count; i++) {
        AstNode *m = cls->methods.items[i];
        if (!strcmp(m->name, name)) return m;
    }
    return NULL;
}

/* Call a function/method with pre-evaluated arg values */
static Value *call_func_with_vals(Env *env, AstNode *func_def, Value **arg_vals, int arg_count, PJObject *self_obj) {
    ScopeTable *saved_scope = env->scope;
    env->scope = scope_new(saved_scope); /* new scope, inherits parent for closures - actually we want fresh scope with globals accessible via env */

    /* Actually for this language, functions don't close over locals; use a fresh scope with parent = NULL (globals via env) */
    /* But we need self to be accessible. Let's parent to saved_scope for now - simple approach */

    int param_offset = 0;
    if (func_def->is_method && func_def->params.count > 0 && !strcmp(func_def->params.items[0], "self")) {
        if (self_obj) {
            scope_set(env->scope, "self", val_obj(self_obj), 0, 1);
        }
        param_offset = 1;
    }

    int expected = func_def->params.count - param_offset;
    if (expected != arg_count) {
        fprintf(stderr, "Runtime Error: Function '%s' expected %d args but got %d\n",
            func_def->name, expected, arg_count);
        exit(1);
    }
    for (int i = 0; i < arg_count; i++) {
        scope_set(env->scope, func_def->params.items[i + param_offset], arg_vals[i], 0, 1);
    }

    g_returning = 0;
    for (int i = 0; i < func_def->body.count && !g_returning; i++) {
        visit(env, func_def->body.items[i], self_obj);
    }

    Value *ret = g_returning ? g_return_val : val_null();
    g_returning = 0;
    g_return_val = NULL;

    /* free scope */
    ScopeTable *old = env->scope;
    env->scope = saved_scope;
    /* We don't deeply free for simplicity */
    free(old);

    return ret;
}

static Value *visit(Env *env, AstNode *node, PJObject *self_obj) {
    if (!node) return val_null();

    switch (node->type) {
        case NODE_LITERAL_INT: return val_int(node->int_val);
        case NODE_LITERAL_STR: return val_str(node->str_val);

        case NODE_TEMPLATE_STR: {
            /* concatenate parts */
            char *result = strdup("");
            for (int i = 0; i < node->template_parts.count; i++) {
                AstNode *part = node->template_parts.items[i];
                Value *pv = visit(env, part, self_obj);
                char *ps = val_to_str(pv);
                char *new_r = malloc(strlen(result) + strlen(ps) + 1);
                strcpy(new_r, result); strcat(new_r, ps);
                free(result); free(ps);
                result = new_r;
            }
            Value *v = val_str(result);
            free(result);
            return v;
        }

        case NODE_VAR_ACCESS: {
            /* check if it's 'self' and we have a self_obj */
            if (self_obj && !strcmp(node->name, "self")) return val_obj(self_obj);
            return scope_get(env->scope, node->name);
        }

        case NODE_VAR_DECL: {
            Value *v = visit(env, node->expr, self_obj);
            scope_set(env->scope, node->name, v, node->is_const, 1);
            return val_null();
        }

        case NODE_ASSIGN: {
            Value *v = visit(env, node->expr, self_obj);
            scope_set(env->scope, node->name, v, 0, 0);
            return val_null();
        }

        case NODE_PRINT: {
            Value *v = visit(env, node->expr, self_obj);
            val_print(v);
            printf("\n");
            return val_null();
        }

        case NODE_READLN: {
            Value *prompt_val = visit(env, node->expr, self_obj);
            char *ps = val_to_str(prompt_val);
            printf("%s", ps); free(ps);
            fflush(stdout);
            char buf[4096];
            if (fgets(buf, sizeof(buf), stdin)) {
                int l = strlen(buf);
                if (l > 0 && buf[l-1] == '\n') buf[l-1] = '\0';
                return val_str(buf);
            }
            return val_str("");
        }
	case NODE_READI: {
	    Value *prompt_val = visit(env, node->expr, self_obj);
            char *ps = val_to_str(prompt_val);
            printf("%s", ps); free(ps);
            fflush(stdout);
            char buf[4096];
            if (fgets(buf, sizeof(buf), stdin)) {
                int l = strlen(buf);
                if (l > 0 && buf[l-1] == '\n') buf[l-1] = '\0';
                char *endptr;
                long val=strtol(buf,&endptr,10);
                if (endptr==buf){
                    fprintf(stderr,"Error: ecpected integer input.\n");
                    return val_int(0);
                }
                return val_int((int)val);
            }
            return val_str(0);
	}
        case NODE_READF: {
            Value  *prompt_val=visit(env,node->expr,self_obj);
            char *ps=val_to_str(prompt_val);
            printf("%s",ps);free(ps);
            fflush(stdout);
            char buf[4069];
            if (fgets(buf,sizeof(buf),stdin)){
		int l=strlen(buf);
                if(l>0 && buf[l-1]=='\n') buf[l-1]='\0';
                char *endptr;
                double val=strtod(buf,&endptr);
		if (endptr==buf){
		    fprintf(stderr,"Error: expected float input.\n");
		    return val_double(0.0);
		}
		return val_double((double)val); 
            }
	    return val_double(0.0);
        }
        case NODE_RETURN: {
            Value *v = visit(env, node->expr, self_obj);
            g_return_val = v;
            g_returning = 1;
            return v;
        }

        case NODE_FUNC_DEF: {
            env_reg_func(env, node->name, node);
            return val_null();
        }

        case NODE_CLASS_DEF: {
            env_reg_class(env, node->name, node);
            return val_null();
        }

        case NODE_FUNC_CALL: {
            /* evaluate args */
            Value **arg_vals = malloc(node->args.count * sizeof(Value*));
            for (int i = 0; i < node->args.count; i++)
                arg_vals[i] = visit(env, node->args.items[i], self_obj);
            AstNode *func_def = env_find_func(env, node->name);
            if (!func_def) {
                /* check if it's a method call on self_obj */
                if (self_obj) {
                    AstNode *m = obj_find_method(self_obj, node->name);
                    if (m) {
                        Value *r = call_func_with_vals(env, m, arg_vals, node->args.count, self_obj);
                        free(arg_vals);
                        return r;
                    }
                }
                fprintf(stderr, "Runtime Error: Undefined function '%s'\n", node->name); exit(1);
            }
            Value *r = call_func_with_vals(env, func_def, arg_vals, node->args.count, NULL);
            free(arg_vals);
            return r;
        }

        case NODE_OBJECT_CREATION: {
            AstNode *cls = env_find_class(env, node->name);
            if (!cls) { fprintf(stderr, "Runtime Error: Undefined class '%s'\n", node->name); exit(1); }

            PJObject *obj = calloc(1, sizeof(PJObject));
            obj->class_name = strdup(node->name);
            obj->class_def = cls;
            /* initialize fields */
            obj->field_cap = cls->fields.count + 4;
            obj->fields = malloc(obj->field_cap * sizeof(PJField));
            obj->field_count = 0;
            for (int i = 0; i < cls->fields.count; i++) {
                AstNode *fd = cls->fields.items[i];
                Value *fv = fd->expr ? visit(env, fd->expr, NULL) : val_null();
                obj->fields[obj->field_count].name = strdup(fd->name);
                obj->fields[obj->field_count].val = fv;
                obj->fields[obj->field_count].is_const = fd->is_const;
                obj->field_count++;
            }

            /* evaluate constructor args */
            Value **arg_vals = malloc(node->args.count * sizeof(Value*));
            for (int i = 0; i < node->args.count; i++)
                arg_vals[i] = visit(env, node->args.items[i], self_obj);

            /* call constructor */
            if (cls->constructor) {
                call_func_with_vals(env, cls->constructor, arg_vals, node->args.count, obj);
            } else if (node->args.count > 0) {
                fprintf(stderr, "Runtime Error: Class '%s' has no constructor but got args\n", node->name);
                exit(1);
            }
            free(arg_vals);
            return val_obj(obj);
        }

        case NODE_MEMBER_ACCESS: {
            Value *obj_val = visit(env, node->object, self_obj);
            /* --- list methods --- */
            if (obj_val->type == VAL_LIST) {
                PJList *lst = obj_val->list;
                if (node->is_call) {
                    Value **avals = malloc(node->args.count * sizeof(Value*));
                    for (int i = 0; i < node->args.count; i++)
                        avals[i] = visit(env, node->args.items[i], self_obj);
                    Value *ret = val_null();
                    if (!strcmp(node->name, "append")) {
                        if (node->args.count != 1) { fprintf(stderr, "append() takes 1 arg\n"); exit(1); }
                        pjlist_push(lst, avals[0]);
                    } else if (!strcmp(node->name, "pop")) {
                        if (lst->count == 0) { fprintf(stderr, "Runtime Error: pop from empty list\n"); exit(1); }
                        int idx = node->args.count == 1 ? (int)avals[0]->ival : lst->count - 1;
                        if (idx < 0) idx = lst->count + idx;
                        if (idx < 0 || idx >= lst->count) { fprintf(stderr, "Runtime Error: pop index out of range\n"); exit(1); }
                        ret = lst->items[idx];
                        for (int i = idx; i < lst->count - 1; i++) lst->items[i] = lst->items[i+1];
                        lst->count--;
                    } else if (!strcmp(node->name, "remove")) {
                        if (node->args.count != 1) { fprintf(stderr, "remove() takes 1 arg\n"); exit(1); }
                        int found = 0;
                        for (int i = 0; i < lst->count; i++) {
                            Value *it = lst->items[i];
                            int eq = (it->type == VAL_STR && avals[0]->type == VAL_STR)
                                ? !strcmp(it->sval, avals[0]->sval)
                                : it->ival == avals[0]->ival;
                            if (eq) {
                                for (int j = i; j < lst->count - 1; j++) lst->items[j] = lst->items[j+1];
                                lst->count--; found = 1; break;
                            }
                        }
                        if (!found) { fprintf(stderr, "Runtime Error: remove: value not in list\n"); exit(1); }
                    } else if (!strcmp(node->name, "len") || !strcmp(node->name, "length")) {
                        ret = val_int(lst->count);
                    } else if (!strcmp(node->name, "contains")) {
                        if (node->args.count != 1) { fprintf(stderr, "contains() takes 1 arg\n"); exit(1); }
                        int found = 0;
                        for (int i = 0; i < lst->count; i++) {
                            Value *it = lst->items[i];
                            int eq = (it->type == VAL_STR && avals[0]->type == VAL_STR)
                                ? !strcmp(it->sval, avals[0]->sval)
                                : it->ival == avals[0]->ival;
                            if (eq) { found = 1; break; }
                        }
                        ret = val_bool(found);
                    } else {
                        fprintf(stderr, "Runtime Error: List has no method '%s'\n", node->name); exit(1);
                    }
                    free(avals);
                    return ret;
                } else {
                    /* field access on list */
                    if (!strcmp(node->name, "len") || !strcmp(node->name, "length"))
                        return val_int(lst->count);
                    fprintf(stderr, "Runtime Error: List has no field '%s'\n", node->name); exit(1);
                }
            }
            if (obj_val->type != VAL_OBJECT) {
                fprintf(stderr, "Runtime Error: Member access on non-object\n"); exit(1);
            }
            PJObject *obj = obj_val->obj;
            if (node->is_call) {
                /* method call */
                AstNode *method = obj_find_method(obj, node->name);
                if (!method) {
                    fprintf(stderr, "Runtime Error: Method '%s' not found on '%s'\n",
                        node->name, obj->class_name); exit(1);
                }
                Value **arg_vals = malloc(node->args.count * sizeof(Value*));
                for (int i = 0; i < node->args.count; i++)
                    arg_vals[i] = visit(env, node->args.items[i], self_obj);
                Value *r = call_func_with_vals(env, method, arg_vals, node->args.count, obj);
                free(arg_vals);
                return r;
            } else {
                /* special case: if 'self' in method, read field */
                /* field access */
                return obj_get_field(obj, node->name);
            }
        }

        case NODE_MEMBER_ASSIGN: {
            Value *obj_val = visit(env, node->object, self_obj);
            if (obj_val->type != VAL_OBJECT) {
                /* check if object is 'self' variable access to self_obj */
                fprintf(stderr, "Runtime Error: Member assignment on non-object\n"); exit(1);
            }
            PJObject *obj = obj_val->obj;
            Value *new_val = visit(env, node->expr, self_obj);
            obj_set_field(obj, node->name, new_val);
            return val_null();
        }

        case NODE_BINARY_OP: {
            Value *lv = visit(env, node->left, self_obj);
            Value *rv = visit(env, node->right, self_obj);
            const char *op = node->str_val;

            /* string concatenation */
            if (!strcmp(op, "+") && (lv->type == VAL_STR || rv->type == VAL_STR)) {
                char *ls = val_to_str(lv), *rs = val_to_str(rv);
                char *res = malloc(strlen(ls) + strlen(rs) + 1);
                strcpy(res, ls); strcat(res, rs);
                Value *v = val_str(res);
                free(ls); free(rs); free(res);
                return v;
            }

            if (!strcmp(op, "+")) return val_int(lv->ival + rv->ival);
            if (!strcmp(op, "-")) return val_int(lv->ival - rv->ival);
            if (!strcmp(op, "*")) return val_int(lv->ival * rv->ival);
            if (!strcmp(op, "/")) {
                if (rv->fval == 0.0) { fprintf(stderr, "Runtime Error: Division by zero\n"); exit(1); }
                return val_double((long double)lv->fval / (long double)rv->fval);
            }
            if (!strcmp(op, "%")) {
                if (rv->ival == 0) { fprintf(stderr, "Runtime Error: Modulo by zero\n"); exit(1); }
                return val_int(lv->ival % rv->ival);
            }
            /* comparison */
            if (!strcmp(op, "==")) {
                if (lv->type == VAL_STR && rv->type == VAL_STR)
                    return val_bool(!strcmp(lv->sval, rv->sval));
                return val_bool(lv->ival == rv->ival);
            }
            if (!strcmp(op, "!=")) {
                if (lv->type == VAL_STR && rv->type == VAL_STR)
                    return val_bool(strcmp(lv->sval, rv->sval) != 0);
                return val_bool(lv->ival != rv->ival);
            }
            if (!strcmp(op, "<"))  return val_bool(lv->ival < rv->ival);
            if (!strcmp(op, ">"))  return val_bool(lv->ival > rv->ival);
            if (!strcmp(op, "<=")) return val_bool(lv->ival <= rv->ival);
            if (!strcmp(op, ">=")) return val_bool(lv->ival >= rv->ival);
            fprintf(stderr, "Runtime Error: Unknown operator '%s'\n", op); exit(1);
        }

        case NODE_UNARY_OP: {
            Value *v = visit(env, node->left, self_obj);
            if (!strcmp(node->str_val, "-")) return val_int(-v->ival);
            else if (!strcmp(node->str_val,"++")) return val_int(v->ival++);
            else if (!strcmp(node->str_val,"--")) return val_int(v->ival--);
            return v;
        }

        case NODE_IF: {
            Value *cond = visit(env, node->expr, self_obj);
            if (val_truthy(cond)) {
                for (int i = 0; i < node->body.count && !g_returning; i++)
                    visit(env, node->body.items[i], self_obj);
            } else {
                int handled = 0;
                for (int ei = 0; ei < node->else_if_count && !handled; ei++) {
                    Value *ec = visit(env, node->else_if_conds.items[ei], self_obj);
                    if (val_truthy(ec)) {
                        NodeList *eib = &node->else_if_bodies[ei];
                        for (int j = 0; j < eib->count && !g_returning; j++)
                            visit(env, eib->items[j], self_obj);
                        handled = 1;
                    }
                }
                if (!handled) {
                    for (int i = 0; i < node->else_body.count && !g_returning; i++)
                        visit(env, node->else_body.items[i], self_obj);
                }
            }
            return val_null();
        }

        case NODE_WHILE: {
            while (!g_returning) {
                Value *cond = visit(env, node->expr, self_obj);
                if (!val_truthy(cond)) break;
                for (int i = 0; i < node->body.count && !g_returning; i++)
                    visit(env, node->body.items[i], self_obj);
            }
            return val_null();
        }

        case NODE_FIELD_DECL:
            return val_null(); /* handled during object creation */

        case NODE_MAIN_DEF:
            fprintf(stderr, "Internal error: visit called on NODE_MAIN_DEF\n"); exit(1);

        case NODE_LIST: {
            PJList *l = pjlist_new();
            for (int i = 0; i < node->args.count; i++)
                pjlist_push(l, visit(env, node->args.items[i], self_obj));
            return val_list(l);
        }

        case NODE_INDEX: {
            Value *lst = visit(env, node->left, self_obj);
            Value *idx = visit(env, node->right, self_obj);
            if (lst->type == VAL_LIST) {
                int i = (int)idx->ival;
                if (i < 0) i = lst->list->count + i;
                if (i < 0 || i >= lst->list->count) {
                    fprintf(stderr, "Runtime Error: List index %d out of range (len=%d)\n", i, lst->list->count);
                    exit(1);
                }
                return lst->list->items[i];
            }
            if (lst->type == VAL_STR) {
                int i = (int)idx->ival;
                if (i < 0) i = (int)strlen(lst->sval) + i;
                if (i < 0 || i >= (int)strlen(lst->sval)) {
                    fprintf(stderr, "Runtime Error: String index out of range\n"); exit(1);
                }
                char buf[2] = {lst->sval[i], 0};
                return val_str(buf);
            }
            fprintf(stderr, "Runtime Error: Cannot index into type %d\n", lst->type); exit(1);
        }

        case NODE_INDEX_ASSIGN: {
            Value *lst = visit(env, node->object, self_obj);
            Value *idx = visit(env, node->left, self_obj);
            Value *newval = visit(env, node->expr, self_obj);
            if (lst->type != VAL_LIST) {
                fprintf(stderr, "Runtime Error: Cannot index-assign on non-list\n"); exit(1);
            }
            int i = (int)idx->ival;
            if (i < 0) i = lst->list->count + i;
            if (i < 0 || i >= lst->list->count) {
                fprintf(stderr, "Runtime Error: List index %d out of range\n", i); exit(1);
            }
            lst->list->items[i] = newval;
            return val_null();
        }

        case NODE_FOR: {
            if (node->is_method) {
                /* for-in: for (let e:T in list) {} */
                Value *iter = visit(env, node->object, self_obj);
                if (iter->type != VAL_LIST) {
                    fprintf(stderr, "Runtime Error: for-in requires a list\n"); exit(1);
                }
                for (int i = 0; i < iter->list->count && !g_returning; i++) {
                    ScopeTable *saved = env->scope;
                    env->scope = scope_new(saved);
                    scope_set(env->scope, node->name, iter->list->items[i], 0, 1);
                    for (int j = 0; j < node->body.count && !g_returning; j++)
                        visit(env, node->body.items[j], self_obj);
                    env->scope = saved;
                }
            } else {
                /* C-style: for (let i=init; cond; post) {} */
                /* declare loop variable in a fresh scope */
                ScopeTable *saved = env->scope;
                env->scope = scope_new(saved);
                scope_set(env->scope, node->name, visit(env, node->left, self_obj), 0, 1);
                while (!g_returning) {
                    Value *cond = visit(env, node->expr, self_obj);
                    if (!val_truthy(cond)) break;
                    /* run body in inner scope */
                    ScopeTable *body_saved = env->scope;
                    env->scope = scope_new(body_saved);
                    for (int i = 0; i < node->body.count && !g_returning; i++)
                        visit(env, node->body.items[i], self_obj);
                    env->scope = body_saved;
                    /* run post step (assigns back into loop scope) */
                    visit(env, node->right, self_obj);
                }
                env->scope = saved;
            }
            return val_null();
        }

        default:
            fprintf(stderr, "Runtime Error: Unknown node type %d\n", node->type); exit(1);
    }
}

static void run(Env *env, AstNode *program) {
    /* First pass: register functions and classes */
    for (int i = 0; i < program->body.count; i++) {
        AstNode *stmt = program->body.items[i];
        if (stmt->type == NODE_FUNC_DEF || stmt->type == NODE_CLASS_DEF)
            visit(env, stmt, NULL);
    }
    /* Second pass: execute statements */
    for (int i = 0; i < program->body.count; i++) {
        AstNode *stmt = program->body.items[i];
        if (stmt->type != NODE_FUNC_DEF && stmt->type != NODE_CLASS_DEF)
            visit(env, stmt, NULL);
        if (g_returning) break;
    }
}

/* ============================================================
 * PACKAGE SYSTEM
 * ============================================================ */

static char *read_file(const char *path) {
    FILE *f = fopen(path, "r");
    if (!f) { fprintf(stderr, "Error: Cannot open file '%s'\n", path); exit(1); }
    fseek(f, 0, SEEK_END);
    long sz = ftell(f); rewind(f);
    char *buf = malloc(sz + 1);
    fread(buf, 1, sz, f);
    buf[sz] = '\0';
    fclose(f);
    return buf;
}

/*
 * Scans tokens for a leading "package <name>;" declaration.
 * Returns heap-allocated package name, or NULL if none found.
 * Only checks the very first non-EOF token(s) — package must be first.
 */
static char *detect_package(TokenList *tl) {
    if (tl->count >= 3 &&
        tl->tokens[0].type == TOK_PACKAGE &&
        tl->tokens[2].type == TOK_SEMICOLON) {
        /* Accept any token as a package name (identifiers, keywords like 'main', etc.) */
        return strdup(tl->tokens[1].value);
    }
    return NULL;
}

/*
 * Parse a package file: starts with "package <name>;" then has top-level
 * func and class definitions (no public class main wrapper needed).
 * Returns a NODE_MAIN_DEF whose body contains only func/class defs.
 */
static AstNode *parse_package_file(Parser *p) {
    /* consume "package <name>;" */
    p_eat(p, TOK_PACKAGE);
    p_advance(p); /* consume the package name regardless of token type */
    p_eat(p, TOK_SEMICOLON);

    AstNode *pkg = new_node(NODE_MAIN_DEF); /* reuse NODE_MAIN_DEF as container */
    nl_init(&pkg->body);

    while (p_cur(p)->type != TOK_EOF) {
        Token *cur = p_cur(p);
        if (cur->type == TOK_DEF) {
            nl_push(&pkg->body, parse_function_definition(p, 0));
        } else if (cur->type == TOK_PUBLIC) {
            p_advance(p);
            if (p_cur(p)->type != TOK_CLASS) {
                fprintf(stderr, "Package Error: expected 'class' after 'public'\n"); exit(1);
            }
            nl_push(&pkg->body, parse_class_definition(p));
        } else if (cur->type == TOK_CLASS) {
            nl_push(&pkg->body, parse_class_definition(p));
        } else {
            fprintf(stderr, "Package Error: only func/class definitions allowed at top level "
                            "in package file, got token '%s'\n", cur->value);
            exit(1);
        }
    }
    return pkg;
}

/*
 * Load a single package file into env (registers its funcs and classes).
 * Returns the package name (heap-alloc) or NULL if file has no package decl.
 */
static char** load_package_file(Env *env, const char *path, int debug, const char *target_pkg) {
    FILE *f = fopen(path, "r");
    if (!f) {
        if (debug) printf("[pjrt] Skipping '%s' (cannot open)\n", path);
        return NULL;
    }
    fseek(f, 0, SEEK_END);
    long sz = ftell(f); rewind(f);
    char *src = malloc(sz + 1);
    fread(src, 1, sz, f);
    src[sz] = '\0';
    fclose(f);

    TokenList tl = tokenize(src);
    free(src);

    char *pkg_name = detect_package(&tl);
    if (!pkg_name) {
        /* No package declaration — skip silently (or verbosely in debug) */
        if (debug) printf("[pjrt] Skipping '%s' (no package declaration)\n", path);
        return NULL;
    }

    /* Only load files whose package name matches the requested package */
    if (target_pkg && strcmp(pkg_name, target_pkg) != 0) {
        if (debug) printf("[pjrt] Skipping '%s' (package '%s' != target '%s')\n", path, pkg_name, target_pkg);
        free(pkg_name);
        return NULL;
    }

    if (debug) printf("[pjrt] Loading package '%s' from '%s'\n", pkg_name, path);

    Parser p; p.tl = &tl; p.idx = 0;
    AstNode *pkg_ast = parse_package_file(&p);

    /* Register all funcs and classes from this package into global env */
    for (int i = 0; i < pkg_ast->body.count; i++) {
        AstNode *node = pkg_ast->body.items[i];
        if (node->type == NODE_FUNC_DEF)  env_reg_func(env, node->name, node);
        if (node->type == NODE_CLASS_DEF) env_reg_class(env, node->name, node);
    }
    char **re = malloc(2 * sizeof(char*));
    re[0] = pkg_name;
    re[1] = strdup(path);
    return re;
}

/* ============================================================
 * MAIN ENTRY POINT
 * ============================================================ */

static void print_usage_pjc(void) {
    printf("pjc usage:\n"
           "  help              show this help page\n"
           "  autorun           autorun all pj files\n"
           "  -d <file>.pj      enable debug\n"
           "  <file>.pj         interpret <file>.pj\n");
}

static void print_usage_pjrt(void) {
    printf("pjrt (PayJar RunTime) usage:\n"
           "  help              show this help page\n"
           "  run <package>     load package from current dir and run main.pj\n"
           "  -d                enable debug mode\n"
           "\n"
           "pjrt scans all .pj files in the current directory for 'package <n>;'\n"
           "declarations and loads them before running main.pj.\n"
           "Files without a package declaration are ignored.\n");
}

static void interpret_file(const char *path, int debug) {
    char *src = read_file(path);

    TokenList tl = tokenize(src);
    free(src);

    /* Reject package files — they cannot be run directly with pjc */
    char *pkg = detect_package(&tl);
    if (pkg) {
        fprintf(stderr,
            "Error: '%s' is a package file (package %s).\n"
            "Cannot run a package with pjc — use pjrt instead.\n",
            path, pkg);
        free(pkg);
        exit(1);
    }

    if (debug) {
        printf("=== TOKENS ===\n");
        for (int i = 0; i < tl.count; i++)
            printf("  [%d] type=%d value=%s\n", i, tl.tokens[i].type, tl.tokens[i].value);
        printf("==============\n");
    }

    Parser p; p.tl = &tl; p.idx = 0;
    AstNode *prog = parse_program(&p);
    if (debug) printf("=== Parse OK ===\n");

    Env env = {0};
    env.scope = scope_new(NULL);

    run(&env, prog);
}

/* ---- pjrt: scan directory for package files ---- */

#include <dirent.h>
#include <stdbool.h>
size_t len(char** arr){
  size_t i=0;
  while (arr[i]!=NULL)i++;
  return i;
}
size_t llen(char*** arr){
 size_t i=0;
 while(arr[i]!=NULL) i++;
 return i;
}
bool isin(char* e,char** list){
  for (int i;i<len(list);i++){
   char* re=list[i];
   if (re==e) return true;
  }
  return false;
}
static void pjrt_run(int debug,char* pkgn) {
    Env env = {0};
    env.scope = scope_new(NULL);

    /* Scan current directory for *.pj files */
    DIR *d = opendir(".");
    if (!d) { fprintf(stderr, "pjrt Error: cannot open current directory\n"); exit(1); }

    int pkg_count = 0;
    struct dirent *entry;
    while ((entry = readdir(d)) != NULL) {
        const char *name = entry->d_name;
        /* skip main.pj — loaded separately */
        if (!strcmp(name, "main.pj")) continue;
        /* only .pj files */
        size_t nlen = strlen(name);
        if (nlen < 4 || strcmp(name + nlen - 3, ".pj") != 0) continue;

        char **pkg = load_package_file(&env, name, debug, pkgn);
        if (pkg) {
            pkg_count++;
            free(pkg[0]);
            free(pkg[1]);
            free(pkg);
        }
        /* files without package declaration or wrong package are silently skipped */
    }
    closedir(d);

    if (pkg_count == 0) {
        fprintf(stderr, "pjrt Error: package '%s' not found in current directory\n", pkgn);
        exit(1);
    }
    if (debug) printf("[pjrt] Loaded file(s) for package: %s.\n", pkgn); 
    /* Now run main.pj */
    char *src = read_file("main.pj");
    TokenList tl = tokenize(src);
    free(src);

    /* main.pj must NOT have a package declaration */
    char *main_pkg = detect_package(&tl);
    if (main_pkg) {
        fprintf(stderr,
            "pjrt Error: main.pj has a package declaration (package %s).\n"
            "main.pj must not declare a package — it is the entry point.\n",
            main_pkg);
        free(main_pkg);
        exit(1);
    }

    if (debug) {
        printf("=== TOKENS (main.pj) ===\n");
        for (int i = 0; i < tl.count; i++)
            printf("  [%d] type=%d value=%s\n", i, tl.tokens[i].type, tl.tokens[i].value);
        printf("========================\n");
    }

    Parser p; p.tl = &tl; p.idx = 0;
    AstNode *prog = parse_program(&p);
    if (debug) printf("=== Parse OK (main.pj) ===\n");

    run(&env, prog);
}
void autorun(int debug){
    DIR *d=opendir(".");
    struct dirent *entry;
    while ((entry = readdir(d)) != NULL) {
        const char *name = entry->d_name;
        size_t nlen = strlen(name);
        if (nlen < 4 || strcmp(name + nlen - 3, ".pj") != 0) continue;
        interpret_file(name,debug);
    }
    closedir(d);
}
int main(int argc, char **argv) {
    /* Determine if we were invoked as 'pjrt' or 'pjc' by checking argv[0] */
    const char *exe = argv[0];
    /* basename-like: find last / or \ */
    const char *base = exe;
    for (const char *p = exe; *p; p++)
        if (*p == '/' || *p == '\\') base = p + 1;

    int is_pjrt = (strncmp(base, "pjrt", 4) == 0);

    if (is_pjrt) {
        /* ---- pjrt mode ---- */
        if (argc < 2) {
            fprintf(stderr, "Not enough arguments. Run 'pjrt help' for help.\n");
            return 1;
        }
        int debug = 0;
        int argi = 1;
        if (!strcmp(argv[argi], "-d")) {
            debug = 1;
            argi++;
        }
        if (argi >= argc) {
            fprintf(stderr, "Not enough arguments. Run 'pjrt help' for help.\n");
            return 1;
        }
        if (!strcmp(argv[argi], "run")) {
            argi++;
            if (argi >= argc) {
                fprintf(stderr, "pjrt Error: 'run' requires a package name argument.\n");
                return 1;
            }
            pjrt_run(debug, argv[argi]);
        } else if (!strcmp(argv[argi], "help")) {
            print_usage_pjrt();
        } else {
            fprintf(stderr, "Command '%s' not found. Run 'pjrt help' for help.\n", argv[argi]);
            return 1;
        }
    } else {
        /* ---- pjc mode ---- */
        if (argc < 2) {
            fprintf(stderr, "Not enough CLI arguments. Run 'pjc help' for help.\n");
            return 1;
        }
        if (!strcmp(argv[1], "help")) {
            print_usage_pjc();
        } else if (!strcmp(argv[1], "autorun")) {
            autorun(0);
        } else if (!strcmp(argv[1], "-d")) {
            if (argc < 3) {
                fprintf(stderr, "Error: -d requires a file argument\n");
                return 1;
            }
            if (!strcmp(argv[2], "autorun")) autorun(1);
            else interpret_file(argv[2], 1);
        } else if (strchr(argv[1], '.')) {
            interpret_file(argv[1], 0);
        } else {
            fprintf(stderr, "Command '%s' not found. Run 'pjc help' for help.\n", argv[1]);
            return 1;
        }
    }
    return 0;
}
