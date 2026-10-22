export type TokenType =
  | 'PUBLIC' | 'PRIVATE' | 'CLASS' | 'MAIN' | 'SELF' | 'INNERSELF'
  | 'DEF' | 'PRINT' | 'PASS' | 'LET' | 'CONST' | 'VAR' | 'NEW' | 'READLN' | 'READI' | 'READF' | 'RETURN'
  | 'IF' | 'ELSE' | 'WHILE' | 'FOR' | 'IN' | 'RANGE' | 'BREAK' | 'CONTINUE'
  | 'IMPORT' | 'PACKAGE' | 'ENUM' | 'TRY' | 'EXCEPT' | 'FINALLY' | 'THROW' | 'ASYNC' | 'AWAIT'
  | 'AND' | 'OR' | 'NOT' | 'AS' | 'INT' | 'FLOAT' | 'STR' | 'NULL' | 'ARRAY' | 'COMPLEX'
  | 'IDENTIFIER' | 'STRING_LITERAL' | 'NUMBER' | 'FLOAT_LITERAL' | 'IMAGINARY_LITERAL' | 'REGEX_STRING' | 'BACKTICK_STRING'
  | 'PLUS' | 'MINUS' | 'MULTIPLY' | 'DIVIDE' | 'MODULO' | 'CARET' | 'PLUS_EQUAL' | 'MINUS_EQUAL' | 'PLUS_PLUS' | 'MINUS_MINUS' | 'ARROW'
  | 'EQUAL_EQUAL' | 'NOT_EQUAL' | 'LESS_THAN' | 'GREATER_THAN' | 'LESS_EQUAL' | 'GREATER_EQUAL'
  | 'EQUAL' | 'LPAREN' | 'RPAREN' | 'LBRACK' | 'RBRACK' | 'LBRACE' | 'RBRACE' | 'COLON' | 'SEMICOLON' | 'AT' | 'COMMA' | 'DOT' | 'AMP' | 'LINE' | 'QUESTION'
  | 'EOF';

export class Token {
  constructor(public type: TokenType, public value: string) {}
  public toString(): string {
    return `Token(${this.type}, ${this.value})`;
  }
}

export interface ASTNode {
    type: string;
    [key: string]: any;
}

export interface MainDefinitionNode extends ASTNode {
    type: "main_definition";
    name: string;
    body: ASTNode[];
}

export interface LiteralNode extends ASTNode {
    type: "literal";
    value: string | number | null;
    data_type: "string" | "number";
    prompt?: string; // Used for READLN expressions during parsing
}


export class Lexer {
  private text: string;
  private pos: number = 0;
  private current_char: string | null;

  constructor(text: string) {
    // The Python implementation included re.sub for comments, which we implement here.
    this.text = this.removeComments(text);
    this.current_char = this.text.length > 0 ? this.text[0] : null;
  }

  private removeComments(text: string): string {
    // Keep comment markers inside quoted strings/template literals intact.
    let out = '';
    let quote: string | null = null;
    for (let i = 0; i < text.length; i++) {
      const ch = text[i];
      if (quote !== null) {
        out += ch;
        if (ch === '\\' && i + 1 < text.length) out += text[++i];
        else if (ch === quote) quote = null;
        continue;
      }
      if (ch === '"' || ch === "'" || ch === '`') { quote = ch; out += ch; continue; }
      if (ch === '/' && text[i + 1] === '/') {
        while (i < text.length && text[i] !== '\n') i++;
        if (i < text.length) out += '\n';
        continue;
      }
      if (ch === '/' && text[i + 1] === '*') {
        i += 2;
        while (i + 1 < text.length && !(text[i] === '*' && text[i + 1] === '/')) i++;
        i++;
        continue;
      }
      out += ch;
    }
    return out;
  }

  private advance(): void {
    this.pos += 1;
    if (this.pos < this.text.length) {
      this.current_char = this.text[this.pos];
    } else {
      this.current_char = null;
    }
  }

  private skipWhitespace(): void {
    while (this.current_char !== null && /\s/.test(this.current_char)) {
      this.advance();
    }
  }

  private peek(offset: number = 1): string | null {
    const peekPos = this.pos + offset;
    if (peekPos < this.text.length) {
      return this.text[peekPos];
    }
    return null;
  }

  private identifier(): string {
    let result = '';
    while (this.current_char !== null && (/[a-zA-Z0-9_]/.test(this.current_char))) {
      result += this.current_char;
      this.advance();
    }
    return result;
  }

  private stringLiteral(quoteChar: string): string {
    let result = '';
    this.advance();
    while (this.current_char !== null && this.current_char !== quoteChar) {
      if (this.current_char === '\\') {
        this.advance();
        if (this.current_char === null) break;
        const escapes: Record<string, string> = { n: '\n', r: '\r', t: '\t', '0': '\0', '\\': '\\', '"': '"', "'": "'", '`': '`' };
        result += escapes[this.current_char] ?? this.current_char;
        this.advance();
      } else {
        result += this.current_char;
        this.advance();
      }
    }
    if (this.current_char !== quoteChar) throw new Error("Lexer Error: Unterminated string literal");
    this.advance();
    return result;
  }

  private backtickStringLiteral(): string {
    return this.stringLiteral('`');
  }

  public getNextToken(): Token | null {
    while (this.current_char !== null) {
      this.skipWhitespace();
      if (this.current_char === null) return null;

      // Identifiers and Keywords
      if (/[a-zA-Z_]/.test(this.current_char)) {
        const id = this.identifier();
        // Match the keyword vocabulary in src/payjar/lexer.rs.
        const literalKeywords: Record<string, string> = { true: '1', false: '0', null: '0' };
        if (id in literalKeywords) return new Token('NUMBER', literalKeywords[id]);
        const keywords: { [key: string]: TokenType } = {
          public: 'PUBLIC', private: 'PRIVATE', class: 'CLASS', enum: 'ENUM',
          main: 'MAIN', self: 'SELF', inner_self: 'INNERSELF', innerSelf: 'INNERSELF',
          func: 'DEF', async: 'ASYNC', await: 'AWAIT', println: 'PRINT', printerr: 'PRINT',
          pass: 'PASS', let: 'LET', const: 'CONST', var: 'VAR', new: 'NEW',
          readln: 'READLN', readi: 'READI', readf: 'READF', return: 'RETURN',
          if: 'IF', else: 'ELSE', while: 'WHILE', for: 'FOR', in: 'IN', range: 'RANGE',
          break: 'BREAK', continue: 'CONTINUE', import: 'IMPORT', package: 'PACKAGE',
          try: 'TRY', except: 'EXCEPT', finally: 'FINALLY', throw: 'THROW',
          and: 'AND', or: 'OR', not: 'NOT', as: 'AS',
          Int: 'INT', Float: 'FLOAT', String: 'STR', Null: 'NULL', Array: 'ARRAY', Complex: 'COMPLEX'
        };
        const type = keywords[id] || 'IDENTIFIER';
        return new Token(type, id);
      }

      // Integer, floating-point and imaginary numeric literals.
      if (/[0-9]/.test(this.current_char)) {
        let number = '';
        while (this.current_char !== null && /[0-9]/.test(this.current_char)) { number += this.current_char; this.advance(); }
        let type: TokenType = 'NUMBER';
        if (this.current_char === '.' && this.peek() !== null && /[0-9]/.test(this.peek()!)) {
          type = 'FLOAT_LITERAL'; number += '.'; this.advance();
          while (this.current_char !== null && /[0-9]/.test(this.current_char)) { number += this.current_char; this.advance(); }
        }
        if (this.current_char === 'i') { type = 'IMAGINARY_LITERAL'; number += 'i'; this.advance(); }
        return new Token(type, number);
      }

      // Operators and punctuation; recognize paired forms first.
      if (this.current_char === '=') {
        if (this.peek() === '=') { this.advance(); this.advance(); return new Token('EQUAL_EQUAL', '=='); }
        this.advance(); return new Token('EQUAL', '=');
      }
      if (this.current_char === '!') {
        if (this.peek() === '=') { this.advance(); this.advance(); return new Token('NOT_EQUAL', '!='); }
        this.advance(); return new Token('NOT', '!');
      }
      if (this.current_char === '<') {
        if (this.peek() === '=') { this.advance(); this.advance(); return new Token('LESS_EQUAL', '<='); }
        this.advance(); return new Token('LESS_THAN', '<');
      }
      if (this.current_char === '>') {
        if (this.peek() === '=') { this.advance(); this.advance(); return new Token('GREATER_EQUAL', '>='); }
        this.advance(); return new Token('GREATER_THAN', '>');
      }
      if (this.current_char === '+') {
        if (this.peek() === '=') { this.advance(); this.advance(); return new Token('PLUS_EQUAL', '+='); }
        if (this.peek() === '+') { this.advance(); this.advance(); return new Token('PLUS_PLUS', '++'); }
        this.advance(); return new Token('PLUS', '+');
      }
      if (this.current_char === '-') {
        if (this.peek() === '=') { this.advance(); this.advance(); return new Token('MINUS_EQUAL', '-='); }
        if (this.peek() === '-') { this.advance(); this.advance(); return new Token('MINUS_MINUS', '--'); }
        if (this.peek() === '>') { this.advance(); this.advance(); return new Token('ARROW', '->'); }
        this.advance(); return new Token('MINUS', '-');
      }
      if (this.current_char === '&') {
        this.advance(); if (this.current_char === '&') { this.advance(); return new Token('AND', '&&'); }
        return new Token('AMP', '&');
      }
      if (this.current_char === '|') {
        this.advance(); if (this.current_char === '|') { this.advance(); return new Token('OR', '||'); }
        return new Token('LINE', '|');
      }
      if (this.current_char === '?') { this.advance(); return new Token('QUESTION', '?'); }
      if (this.current_char === '^') { this.advance(); return new Token('CARET', '^'); }
      if (this.current_char === '*') { this.advance(); return new Token('MULTIPLY', '*'); }
      if (this.current_char === '/') { this.advance(); return new Token('DIVIDE', '/'); }
      if (this.current_char === '%') { this.advance(); return new Token('MODULO', '%'); }
      if (this.current_char === '(') { this.advance(); return new Token('LPAREN', '('); }
      if (this.current_char === ')') { this.advance(); return new Token('RPAREN', ')'); }
      if (this.current_char === '[') { this.advance(); return new Token('LBRACK', '['); }
      if (this.current_char === ']') { this.advance(); return new Token('RBRACK', ']'); }
      if (this.current_char === '{') { this.advance(); return new Token('LBRACE', '{'); }
      if (this.current_char === '}') { this.advance(); return new Token('RBRACE', '}'); }
      if (this.current_char === ';') { this.advance(); return new Token('SEMICOLON', ';'); }
      if (this.current_char === ',') { this.advance(); return new Token('COMMA', ','); }
      if (this.current_char === '.') { this.advance(); return new Token('DOT', '.'); }
      if (this.current_char === ':') { this.advance(); return new Token('COLON', ':'); }
      if (this.current_char === '@') { this.advance(); return new Token('AT', '@'); }

      // String and Backtick Literals
      if (this.current_char === '"' || this.current_char === "'") {
        return new Token('STRING_LITERAL', this.stringLiteral(this.current_char));
      }
      if (this.current_char === '`') {
        return new Token('BACKTICK_STRING', this.backtickStringLiteral());
      }

      throw new Error(`Lexer Error: Invalid character: ${this.current_char}`);
    }
    return null; // EOF
  }

  public tokenize(): Token[] {
    const tokens: Token[] = [];
    let token: Token | null = this.getNextToken();
    while (token) {
      tokens.push(token);
      token = this.getNextToken();
    }
    return tokens;
  }
}
export class Parser {
    private tokens: Token[];
    private current_token: Token | null = null;
    private token_index: number = -1;

    constructor(tokens: Token[]) {
        this.tokens = tokens;
        this.advance();
    }

    private advance(): void {
        this.token_index += 1;
        this.current_token = this.token_index < this.tokens.length ? this.tokens[this.token_index] : null;
    }

    private eat(token_type: TokenType): void {
        if (this.current_token && this.current_token.type === token_type) {
            this.advance();
        } else {
            throw new Error(`Syntax Error: Expected ${token_type}, but got ${this.current_token?.type ?? 'EOF'}. Index: ${this.token_index}`);
        }
    }

    // --- Main Parsing Logic ---
    public parse(): MainDefinitionNode {
        return this.parseMain();
    }

    private parseMain(): MainDefinitionNode {
        const declarations: ASTNode[] = [];
        let main: MainDefinitionNode | null = null;
        while (this.current_token) {
            if (this.current_token.type === 'DEF') {
                declarations.push(this.parseFunctionDefinition(false));
            } else if (this.current_token.type === 'PUBLIC' || this.current_token.type === 'PRIVATE' || this.current_token.type === 'CLASS') {
                const isMain = (this.current_token.type === 'PUBLIC' &&
                    this.tokens[this.token_index + 1]?.type === 'CLASS' &&
                    this.tokens[this.token_index + 2]?.type === 'MAIN') ||
                    (this.current_token.type === 'CLASS' && this.tokens[this.token_index + 1]?.type === 'MAIN');
                if (isMain) {
                    if (main) throw new Error("Syntax Error: Only one 'public class main(@self)' entry point is allowed.");
                    if (this.current_token.type === 'PUBLIC') this.eat('PUBLIC');
                    this.eat('CLASS');
                    this.eat('MAIN');
                    this.eat('LPAREN'); this.eat('AT'); this.eat('SELF'); this.eat('RPAREN');
                    this.eat('LBRACE');
                    const body = this.parseMainBody();
                    this.eat('RBRACE');
                    main = { type: "main_definition", name: 'main', body };
                } else {
                    declarations.push(this.parseClassDefinition());
                }
            } else {
                throw new Error(`Syntax Error: Expected a top-level function, class, or 'public class main(@self)', got ${this.current_token.type}.`);
            }
        }
        if (!main) throw new Error("Syntax Error: Missing 'public class main(@self)' entry point.");
        // Hoist top-level functions and classes before the entry-point statements.
        return { ...main, body: [...declarations, ...main.body] };
    }

    private parseMainBody(): ASTNode[] {
        const statements: ASTNode[] = [];
        while (this.current_token && this.current_token.type !== 'RBRACE') {
            if (this.current_token.type === 'IF') {
                statements.push(this.parseIfStatement());
            } else if (this.current_token.type === 'WHILE') {
                statements.push(this.parseWhileStatement());
            } else if (this.current_token.type === 'PRINT') {
                statements.push(this.parsePrintStatement());
            } else if (['VAR', 'CONST', 'LET'].includes(this.current_token.type)) {
                statements.push(this.parseVariableDeclaration());
            } else if (this.current_token.type === 'DEF') {
                statements.push(this.parseFunctionDefinition());
            } else if (this.current_token.type === 'PUBLIC' || this.current_token.type === 'CLASS') {
                statements.push(this.parseClassDefinition());
            } else if (this.current_token.type === 'SELF') {
                const nextToken = this.tokens[this.token_index + 1];
                if (nextToken?.type !== 'DOT') throw new Error("Syntax Error: 'self' must access a class member.");
                this.eat('SELF');
                statements.push(this.parseMemberAccess({ type: "variable_access", name: "self" }));
            } else if (this.current_token.type === 'IDENTIFIER') {
                const nextToken = this.tokens[this.token_index + 1];
                if (nextToken?.type === 'EQUAL') {
                    statements.push(this.parseAssignmentStatement());
                } else if (nextToken?.type === 'DOT') {
                    const objName = this.current_token.value;
                    this.eat('IDENTIFIER');
                    const objExpr: ASTNode = { type: "variable_access", name: objName };
                    const expr = this.parseMemberAccess(objExpr);
                    statements.push(expr);
                    if (expr.type === "member_access" && expr.is_call) {
                        this.eat('SEMICOLON');
                    }
                } else if (nextToken?.type === 'LPAREN') {
                    const expr = this.parseFunctionCallExpression();
                    statements.push(expr);
                    this.eat('SEMICOLON');
                } else {
                    throw new Error(`Syntax Error: Unexpected token in main body: ${this.current_token.type}`);
                }
            } else {
                throw new Error(`Syntax Error: Unexpected token in main body: ${this.current_token.type}`);
            }
        }
        return statements;
    }
    
    private parseIfStatement(): ASTNode {
        this.eat('IF');
        this.eat('LPAREN');
        const condition = this.parseExpression();
        this.eat('RPAREN');
        this.eat('LBRACE');
        const body = this.parseMainBody();
        this.eat('RBRACE');
        let elseBody: ASTNode[] = [];
        let elseIf: { condition: ASTNode; body: ASTNode[] }[] = [];
        if (this.current_token?.type === 'ELSE') {
            this.eat('ELSE');
            if (this.current_token?.type === 'IF') {
                const nested = this.parseIfStatement();
                elseIf.push({ condition: nested.condition, body: nested.body });
                elseIf.push(...(nested.else_if || []));
                elseBody = nested.else_body || [];
            } else {
                this.eat('LBRACE');
                elseBody = this.parseMainBody();
                this.eat('RBRACE');
            }
        }
        return { type: 'if_statement', condition, body, else_if: elseIf, else_body: elseBody };
    }

    private parseWhileStatement(): ASTNode {
        this.eat('WHILE');
        this.eat('LPAREN');
        const condition = this.parseExpression();
        this.eat('RPAREN');
        this.eat('LBRACE');
        const body = this.parseMainBody();
        this.eat('RBRACE');
        return { type: 'while_statement', condition, body };
    }

    // --- Utility Parsing Functions ---

    private parsePrintStatement(): ASTNode {
        this.eat('PRINT');
        this.eat('LPAREN');
        const expression = this.parseExpression();
        this.eat('RPAREN');
        this.eat('SEMICOLON');
        return { type: "print_statement", expression };
    }

    private parseInputStatement(): LiteralNode {
        this.eat('READLN');
        this.eat('LPAREN');
        // In the parser, we just capture the prompt expression structure
        const promptExpr = this.parseExpression() as LiteralNode;
        this.eat('RPAREN');

        // Return a distinct readln node so the Interpreter can handle interactive input.
        return {
            type: "readln",
            prompt: promptExpr.value as string
        } as any;
    }
    
    private parseVariableDeclaration(): ASTNode {
        const declarationType = this.current_token!.type;
        this.advance();
        const variableName = this.current_token!.value;
        this.eat('IDENTIFIER');
        if (this.current_token?.type === 'COLON') {
            this.eat('COLON');
            if (!this.current_token) throw new Error("Syntax Error: Expected variable type after ':'.");
            this.advance(); // Type annotation is informational in the browser runtime.
        }
        this.eat('EQUAL');
        const valueExpression = this.parseExpression();
        this.eat('SEMICOLON');
        return { type: "variable_declaration", kind: declarationType, name: variableName, value: valueExpression };
    }

    private parseAssignmentStatement(): ASTNode {
        const variableName = this.current_token!.value;
        this.eat('IDENTIFIER');
        this.eat('EQUAL');
        const valueExpression = this.parseExpression();
        this.eat('SEMICOLON');
        return { type: "assignment_statement", name: variableName, value: valueExpression };
    }

    // --- Expression Parsing (Recursive Descent with Precedence) ---

    // Entry point: Handles comparison (lowest precedence)
    private parseExpression(): ASTNode {
        return this.parseOrExpression();
    }

    private parseOrExpression(): ASTNode {
        let left = this.parseAndExpression();
        while (this.current_token && this.current_token.type === 'OR') {
            const operator = this.current_token.value; this.advance();
            left = { type: "binary_op", operator, left, right: this.parseAndExpression() };
        }
        return left;
    }

    private parseAndExpression(): ASTNode {
        let left = this.parseComparisonExpression();
        while (this.current_token && this.current_token.type === 'AND') {
            const operator = this.current_token.value; this.advance();
            left = { type: "binary_op", operator, left, right: this.parseComparisonExpression() };
        }
        return left;
    }

    private parseComparisonExpression(): ASTNode {
        let left = this.parseTerm();
        while (this.current_token && ['EQUAL_EQUAL', 'NOT_EQUAL', 'LESS_THAN', 'GREATER_THAN', 'LESS_EQUAL', 'GREATER_EQUAL'].includes(this.current_token.type)) {
            const operator = this.current_token.value; this.advance();
            left = { type: "binary_op", operator, left, right: this.parseTerm() };
        }
        return left;
    }

    // Addition/Subtraction
    private parseTerm(): ASTNode {
        let left = this.parseFactor();
        while (this.current_token && ['PLUS', 'MINUS'].includes(this.current_token.type)) {
            const operator = this.current_token.value;
            this.advance();
            const right = this.parseFactor();
            left = { type: "binary_op", operator, left, right };
        }
        return left;
    }

    // Multiplication/Division/Modulo and Unary
    private parseFactor(): ASTNode {
        // Unary Plus/Minus
        if (this.current_token && ['PLUS', 'MINUS', 'NOT'].includes(this.current_token.type)) {
            const operator = this.current_token.value; this.advance();
            const operand = this.parseFactor();
            return { type: "unary_op", operator, operand };
        }

        let left = this.parsePrimaryExpression();
        while (this.current_token && ['MULTIPLY', 'DIVIDE', 'MODULO'].includes(this.current_token.type)) {
            const operator = this.current_token.value;
            this.advance();
            const right = this.parsePrimaryExpression();
            left = { type: "binary_op", operator, left, right };
        }
        return left;
    }

    // Literals, Identifiers, Calls, New, Parenthesized (highest precedence)
    private parsePrimaryExpression(): ASTNode {
        if (this.current_token!.type === 'STRING_LITERAL') {
            const value = this.current_token!.value;
            this.eat('STRING_LITERAL');
            return { type: "literal", value, data_type: "string" };
        } else if (this.current_token!.type === 'NUMBER' || this.current_token!.type === 'FLOAT_LITERAL') {
            const tokenType = this.current_token!.type;
            const value = tokenType === 'FLOAT_LITERAL' ? parseFloat(this.current_token!.value) : parseInt(this.current_token!.value, 10);
            this.eat(tokenType);
            return { type: "literal", value, data_type: "number" };
        } else if (this.current_token!.type === 'BACKTICK_STRING') {
            const stringContent = this.current_token!.value;
            this.eat('BACKTICK_STRING');
            const templateParts: ASTNode[] = [];
            // Simplified regex matching for ${variableName} using JS RegExp
            const parts = stringContent.split(/(\$\{(\w+)\})/g).filter(p => p !== '');

            for (let i = 0; i < parts.length; i++) {
                const part = parts[i];
                if (part.startsWith('${') && part.endsWith('}')) {
                    const varName = part.substring(2, part.length - 1);
                    templateParts.push({ type: "variable_access", name: varName });
                    i++; // Skip the variable name part if using the regex capture group trick
                } else if (i % 2 === 0) { // Simple literal split (less robust than the Python original, but simpler in JS)
                    if (part) templateParts.push({ type: "literal", value: part, data_type: "string" });
                }
            }
            return { type: "template_string", parts: templateParts };
        } else if (this.current_token!.type === 'SELF') {
            this.eat('SELF');
            const selfExpr: ASTNode = { type: "variable_access", name: "self" };
            return this.current_token?.type === 'DOT' ? this.parseMemberAccess(selfExpr) : selfExpr;
        } else if (this.current_token!.type === 'IDENTIFIER') {
            const name = this.current_token!.value;
            this.advance();
            const nextToken = this.current_token;

            if (nextToken?.type === 'LPAREN') {
                return this.parseFunctionCallExpression(name);
            } else if (nextToken?.type === 'DOT') {
                const objExpr: ASTNode = { type: "variable_access", name };
                return this.parseMemberAccess(objExpr);
            } else {
                return { type: "variable_access", name };
            }
        } else if (this.current_token!.type === 'READLN') {
            return this.parseInputStatement();
        } else if (this.current_token!.type === 'NEW') {
            return this.parseNewExpression();
        } else if (this.current_token!.type === 'LPAREN') {
            this.eat('LPAREN');
            const expr = this.parseExpression();
            this.eat('RPAREN');
            return expr;
        } else {
            throw new Error(`Syntax Error: Unexpected token in primary expression: ${this.current_token?.type ?? 'EOF'}`);
        }
    }
    
    private parseArguments(): ASTNode[] {
        const argumentsList: ASTNode[] = [];
        if (this.current_token && this.current_token.type !== 'RPAREN') {
            argumentsList.push(this.parseExpression());
            while (this.current_token && this.current_token.type === 'COMMA') {
                this.eat('COMMA');
                argumentsList.push(this.parseExpression());
            }
        }
        return argumentsList;
    }

    private parseFunctionCallExpression(name?: string): ASTNode {
        const functionName = name || this.current_token!.value;
        if (!name) this.eat('IDENTIFIER'); // Consume if not already consumed by parsePrimaryExpression
        
        this.eat('LPAREN');
        const argumentsList = this.parseArguments();
        this.eat('RPAREN');
        return { type: "function_call", name: functionName, arguments: argumentsList };
    }

    private parseMemberAccess(objExpr: ASTNode): ASTNode {
        this.eat('DOT');
        const memberName = this.current_token!.value;
        this.eat('IDENTIFIER');
        let isCall = false;
        let memberArgs: ASTNode[] = [];

        if (this.current_token && this.current_token.type === 'LPAREN') {
            isCall = true;
            this.eat('LPAREN');
            memberArgs = this.parseArguments();
            this.eat('RPAREN');
        }

        // Check for assignment (e.g., obj.property = value;)
        if (this.current_token && this.current_token.type === 'EQUAL') {
            this.eat('EQUAL');
            const valueExpr = this.parseExpression();
            this.eat('SEMICOLON');
            return { type: "member_assignment", object: objExpr, member: memberName, value: valueExpr };
        }

        // If not an assignment, it's an access or a method call
        return { type: "member_access", object: objExpr, member: memberName, is_call: isCall, arguments: memberArgs };
    }

    private parseNewExpression(): ASTNode {
        this.eat('NEW');
        const className = this.current_token!.value;
        this.eat('IDENTIFIER');
        this.eat('LPAREN');
        const argumentsList = this.parseArguments();
        this.eat('RPAREN');
        return { type: "object_creation", class_name: className, arguments: argumentsList };
    }

    // --- Class/Function Definition Logic (methods omitted for space, but follow Python logic) ---
    // The Python implementation for parse_class_definition, parse_function_definition, etc., 
    // are directly convertible but are lengthy. Their structures remain the same.
    private parseClassDefinition(): ASTNode {
        if (this.current_token?.type === 'PUBLIC' || this.current_token?.type === 'PRIVATE') this.advance();
        this.eat('CLASS');
        const className = this.current_token!.value;
        this.eat('IDENTIFIER');
        this.eat('LPAREN');
        this.eat('AT');
        this.eat('INNERSELF');
        this.eat('RPAREN');
        this.eat('LBRACE');
        
        // Simplified body parsing
        const members: ASTNode[] = [];
        const methods: ASTNode[] = [];
        let constructor: ASTNode | null = null;
        
        while (this.current_token && this.current_token.type !== 'RBRACE') {
            if (this.current_token.type === 'PUBLIC' || this.current_token.type === 'PRIVATE') {
                this.advance(); // Visibility modifier; the browser runtime currently enforces syntax, not access control.
                if (this.current_token?.type === 'DEF') {
                    const methodDef = this.parseFunctionDefinition(true);
                    if (methodDef.name === 'init') constructor = methodDef;
                    else methods.push(methodDef);
                } else if (['CONST', 'LET', 'VAR'].includes(this.current_token?.type ?? '')) {
                    members.push(this.parseClassFieldDeclaration());
                } else {
                    throw new Error(`Syntax Error: Expected a field or method after visibility modifier, got ${this.current_token?.type ?? 'EOF'}.`);
                }
            } else if (['CONST', 'LET', 'VAR'].includes(this.current_token.type)) {
                members.push(this.parseClassFieldDeclaration());
            } else if (this.current_token.type === 'DEF') {
                const methodDef = this.parseFunctionDefinition(true); // is_method = true
                if (methodDef.name === 'init') {
                    constructor = methodDef;
                } else {
                    methods.push(methodDef);
                }
            } else {
                throw new Error(`Syntax Error: Unexpected token in class body: ${this.current_token.type}`);
            }
        }
        this.eat('RBRACE');
        return { type: "class_definition", name: className, fields: members, methods: methods, constructor: constructor };
    }

    private parseClassFieldDeclaration(): ASTNode {
        const declarationType = this.current_token!.type;
        this.advance(); // Consume declaration keyword
        const fieldName = this.current_token!.value;
        this.eat('IDENTIFIER');
        if (this.current_token?.type === 'COLON') {
            this.eat('COLON');
            if (!this.current_token) throw new Error("Syntax Error: Expected field type after ':'.");
            this.advance(); // Type annotation is currently informational in the browser runtime.
        }
        let initialValue: ASTNode | null = null;
        if (this.current_token && this.current_token.type === 'EQUAL') {
            this.eat('EQUAL');
            initialValue = this.parseExpression();
        }
        this.eat('SEMICOLON');
        return { type: "field_declaration", kind: declarationType, name: fieldName, value: initialValue };
    }
    
    private parseFunctionDefinition(isMethod: boolean = false): ASTNode {
        this.eat('DEF');
        const functionName = this.current_token!.value;
        this.eat('IDENTIFIER');
        this.eat('LPAREN');

        const parameters: string[] = [];
        if (this.current_token && this.current_token.type !== 'RPAREN') {
            while (this.current_token && this.current_token.type !== 'RPAREN') {
                if (this.current_token.type === 'SELF') {
                    if (!isMethod) throw new Error("Syntax Error: The 'self' parameter is only allowed in class method definitions.");
                    parameters.push('self');
                    this.eat('SELF');
                } else {
                    if (this.current_token.type !== 'IDENTIFIER') throw new Error(`Syntax Error: Expected parameter name, got ${this.current_token.type}.`);
                    parameters.push(this.current_token.value);
                    this.eat('IDENTIFIER');
                    if (this.current_token?.type === 'COLON') {
                        this.eat('COLON');
                        if (!this.current_token) throw new Error("Syntax Error: Expected parameter type after ':'.");
                        this.advance(); // Type annotation is informational in the browser runtime.
                    }
                }
                if (this.current_token?.type === 'COMMA') this.eat('COMMA');
                else break;
            }
        }
        this.eat('RPAREN');
        if (this.current_token?.type === 'ARROW') {
            this.eat('ARROW');
            if (!this.current_token) throw new Error("Syntax Error: Expected return type after '->'.");
            this.advance(); // Return type annotation is informational in the browser runtime.
        }
        this.eat('LBRACE');

        const body: ASTNode[] = [];
        while (this.current_token && this.current_token.type !== 'RBRACE') {
            if (this.current_token.type === 'RETURN') {
                body.push(this.parseReturnStatement());
            } 
            // The body parsing logic should mirror parseMainBody() for local statements
            else if (this.current_token.type === 'IF') { body.push(this.parseIfStatement()); }
            else if (this.current_token.type === 'WHILE') { body.push(this.parseWhileStatement()); }
            else if (this.current_token.type === 'PRINT') {body.push(this.parsePrintStatement());}
            else if (['CONST', 'LET', 'VAR'].includes(this.current_token.type)) {body.push(this.parseVariableDeclaration());}
            else if (this.current_token.type === 'SELF') {
                const nextToken = this.tokens[this.token_index + 1];
                if (nextToken?.type !== 'DOT') throw new Error("Syntax Error: 'self' must access a class member.");
                this.eat('SELF');
                body.push(this.parseMemberAccess({ type: "variable_access", name: "self" }));
            }
            else if (this.current_token.type === 'IDENTIFIER') {
                const nextToken = this.tokens[this.token_index + 1];
                if (nextToken?.type === 'EQUAL') {body.push(this.parseAssignmentStatement());}
                else if (nextToken?.type === 'DOT') {
                   // Member access/assignment logic (requires lookahead/re-implementation)
                   const objName = this.current_token.value;
                   this.eat('IDENTIFIER'); 
                   const objExpr: ASTNode = { type: "variable_access", name: objName };
                   const expr = this.parseMemberAccess(objExpr); 
                   body.push(expr);
                   if (expr.type === "member_access" && expr.is_call) {this.eat('SEMICOLON');}
                }
                 else if (nextToken?.type === 'LPAREN') {
                   const expr = this.parseFunctionCallExpression();
                   body.push(expr);
                   this.eat('SEMICOLON');
               } else {throw new Error(`Syntax Error: Unexpected token in function body: ${this.current_token.type}`);}
            }
            else {throw new Error(`Syntax Error: Unexpected token in function body: ${this.current_token.type}`);}
        }
        this.eat('RBRACE');
        return { type: "function_definition", name: functionName, parameters, body, is_method: isMethod };
    }
    
    private parseReturnStatement(): ASTNode {
        this.eat('RETURN');
        const expression = this.parseExpression();
        this.eat('SEMICOLON');
        return { type: "return_statement", expression };
    }
}


// ====================================================================
// 4. Interpreter Structures
// ====================================================================

export class FunctionReturn extends Error {
    constructor(public value: any) {
        super("FunctionReturn: Internal exception for control flow.");
        this.name = "FunctionReturn";
    }
}

export class PayJarObject {
    // fields: { [key: string]: { value: any, kind: string } }
    // methods: { [key: string]: ASTNode }
    constructor(public class_name: string, public fields: any, public methods: any) {}
    public toString(): string {
        const fieldStr = Object.entries(this.fields).map(([name, data]: [string, any]) => `${name}=${data.value}`).join(', ');
        return `<Object ${this.class_name} (${fieldStr})>`;
    }
}

// ====================================================================
// 5. Interpreter
// ====================================================================

export class Interpreter {
    // scopes: { [key: string]: { value: any, kind: string } }[]
    private scopes: any[] = [{}];
    private functions: { [key: string]: ASTNode } = {};
    private user_defined_classes: { [key: string]: ASTNode } = {};
    // Collected outputs produced by PRINT/println
    private outputs: string[] = [];
    // Optional input provider used for READLN. If provided, it's called with the prompt and should return a string.
    private inputProvider?: (prompt?: string) => string;

    constructor(inputProvider?: (prompt?: string) => string) {
        this.inputProvider = inputProvider;
    }

    public getOutputs(): string[] {
        return this.outputs.slice();
    }

    private get current_scope(): any {
        return this.scopes[this.scopes.length - 1];
    }

    private push_scope(scope_dict: any = {}): void {
        this.scopes.push(scope_dict);
    }

    private pop_scope(): void {
        if (this.scopes.length > 1) {
            this.scopes.pop();
        } else {
            throw new Error("Runtime Error: Cannot pop global scope.");
        }
    }

    // --- Variable Access and Assignment ---

    private getVariable(name: string): { value: any, kind: string } {
        for (let i = this.scopes.length - 1; i >= 0; i--) {
            if (name in this.scopes[i]) {
                return this.scopes[i][name];
            }
        }
        throw new Error(`Runtime Error: Undefined variable '${name}'`);
    }

    private setVariable(name: string, value: any, kind: string | null = null, declareIfNotExist: boolean = false): any {
        if (kind) { // Declaration
            if (name in this.current_scope) {
                throw new Error(`Runtime Error: Redeclaration of variable '${name}' is not allowed in this scope.`);
            }
            this.current_scope[name] = { value: value, kind: kind };
        } else { // Assignment
            for (let i = this.scopes.length - 1; i >= 0; i--) {
                if (name in this.scopes[i]) {
                    if (this.scopes[i][name].kind === 'CONST') {
                        throw new Error(`Runtime Error: Cannot assign to a constant variable '${name}'.`);
                    }
                    this.scopes[i][name].value = value;
                    return value;
                }
            }
            if (declareIfNotExist) {
                this.current_scope[name] = { value: value, kind: "LET" };
            } else {
                throw new Error(`Runtime Error: Assignment to undefined variable '${name}'.`);
            }
        }
        return value;
    }

    // --- Interpreter Entry Point ---

    public interpret(ast: MainDefinitionNode): void {
        if (ast.type !== "main_definition") {
            throw new Error(`Runtime Error: Unsupported AST type for interpretation: ${ast.type}`);
        }

        // First pass: Hoisting (functions and classes)
        for (const statement of ast.body) {
            if (statement.type === "function_definition") {
                this.visitFunctionDefinition(statement);
            } else if (statement.type === "class_definition") {
                this.visitClassDefinition(statement);
            }
        }

        // Second pass: Execute main body statements
        for (const statement of ast.body) {
            if (statement.type !== "function_definition" && statement.type !== "class_definition") {
                try {
                    this.visit(statement);
                } catch (e) {
                    if (e instanceof FunctionReturn) {
                        console.warn(`Warning: Return statement encountered in main body. Value: ${e.value}`);
                    } else {
                        throw e;
                    }
                }
            }
        }
    }

    // --- Visit Dispatcher ---

    private visit(node: ASTNode, current_instance: PayJarObject | null = null): any {
        switch (node.type) {
            case "print_statement": return this.visitPrintStatement(node);
            case "if_statement": return this.visitIfStatement(node, current_instance);
            case "while_statement": return this.visitWhileStatement(node, current_instance);
            case "readln": return this.visitReadln(node);
            case "variable_declaration": return this.visitVariableDeclaration(node);
            case "assignment_statement": return this.visitAssignmentStatement(node);
            case "literal": return (node as LiteralNode).value;
            case "variable_access": return this.visitVariableAccess(node);
            case "template_string": return this.visitTemplateString(node);
            case "function_definition": return this.visitFunctionDefinition(node);
            case "function_call": return this.visitFunctionCall(node, current_instance);
            case "return_statement": return this.visitReturnStatement(node);
            case "class_definition": return this.visitClassDefinition(node);
            case "object_creation": return this.visitObjectCreation(node);
            case "member_access": return this.visitMemberAccess(node, current_instance);
            case "member_assignment": return this.visitMemberAssignment(node, current_instance);
            case "binary_op": return this.visitBinaryOp(node);
            case "unary_op": return this.visitUnaryOp(node);
            case "field_declaration": return; // Handled during object creation
            default: throw new Error(`Runtime Error: Unknown AST node type: ${node.type}`);
        }
    }

    // --- Visit Implementations ---

    private visitIfStatement(node: ASTNode, currentInstance: PayJarObject | null): any {
        const truthy = Boolean(this.visit(node.condition, currentInstance));
        if (truthy) {
            this.push_scope();
            try { for (const statement of node.body) this.visit(statement, currentInstance); }
            finally { this.pop_scope(); }
            return;
        }
        for (const branch of node.else_if || []) {
            if (Boolean(this.visit(branch.condition, currentInstance))) {
                this.push_scope();
                try { for (const statement of branch.body) this.visit(statement, currentInstance); }
                finally { this.pop_scope(); }
                return;
            }
        }
        if (node.else_body?.length) {
            this.push_scope();
            try { for (const statement of node.else_body) this.visit(statement, currentInstance); }
            finally { this.pop_scope(); }
        }
    }

    private visitWhileStatement(node: ASTNode, currentInstance: PayJarObject | null): void {
        // Guard against a runaway loop locking up the browser tab.
        let iterations = 0;
        while (Boolean(this.visit(node.condition, currentInstance))) {
            if (++iterations > 1_000_000) throw new Error('Runtime Error: Loop iteration limit exceeded (1,000,000).');
            this.push_scope();
            try { for (const statement of node.body) this.visit(statement, currentInstance); }
            finally { this.pop_scope(); }
        }
    }

    private visitPrintStatement(node: ASTNode): void {
        const valueToPrint = this.visit(node.expression);
        // Record output and also forward to console for devtools visibility
        try { this.outputs.push(typeof valueToPrint === 'object' ? JSON.stringify(valueToPrint) : String(valueToPrint)); }
        catch { this.outputs.push(String(valueToPrint)); }
        console.log(valueToPrint);
    }

    private visitReadln(node: ASTNode): string {
        const prompt = node.prompt ?? '';
        let value = '';
        if (this.inputProvider) {
            try {
                value = this.inputProvider(prompt);
            } catch (e) {
                value = '';
            }
        } else if (typeof window !== 'undefined' && typeof (window as any).prompt === 'function') {
            const res = (window as any).prompt(prompt || '');
            value = res === null || res === undefined ? '' : String(res);
        } else {
            // No provider and no prompt available (e.g., non-browser environment): return empty string
            value = '';
        }
        return value;
    }
    
    private visitVariableDeclaration(node: ASTNode): void {
        const varName = node.name;
        const varKind = node.kind;
        const varValue = this.visit(node.value);
        this.setVariable(varName, varValue, varKind);
    }

    private visitAssignmentStatement(node: ASTNode): void {
        const varName = node.name;
        const newValue = this.visit(node.value);
        this.setVariable(varName, newValue);
    }

    private visitVariableAccess(node: ASTNode): any {
        return this.getVariable(node.name).value;
    }

    private visitTemplateString(node: ASTNode): string {
        let resultString = "";
        for (const part of node.parts) {
            if (part.type === "literal") {
                resultString += part.value;
            } else if (part.type === "variable_access") {
                const variableValue = this.getVariable(part.name).value;
                resultString += String(variableValue);
            } else {
                throw new Error(`Runtime Error: Unexpected part in template string: ${part.type}`);
            }
        }
        return resultString;
    }

    private visitFunctionDefinition(node: ASTNode): void {
        const funcName = node.name;
        if (!node.is_method && funcName in this.functions) {
            throw new Error(`Runtime Error: Function '${funcName}' already defined globally.`);
        }
        this.functions[funcName] = node;
    }
    
    private visitFunctionCall(node: ASTNode, currentInstance: PayJarObject | null): any {
        const funcName = node.name;
        const providedArgsNodes = node.arguments as ASTNode[];
        if (funcName === 'toStr') {
            if (providedArgsNodes.length !== 1) throw new Error("Runtime Error: toStr expects exactly one argument.");
            return String(this.visit(providedArgsNodes[0], currentInstance));
        }
        if (funcName === 'toInt') {
            if (providedArgsNodes.length !== 1) throw new Error("Runtime Error: toInt expects exactly one argument.");
            const value = Number(this.visit(providedArgsNodes[0], currentInstance));
            if (!Number.isFinite(value)) throw new Error("Runtime Error: toInt received a non-numeric value.");
            return Math.trunc(value);
        }
        if (funcName === 'toFloat') {
            if (providedArgsNodes.length !== 1) throw new Error("Runtime Error: toFloat expects exactly one argument.");
            const value = Number(this.visit(providedArgsNodes[0], currentInstance));
            if (!Number.isFinite(value)) throw new Error("Runtime Error: toFloat received a non-numeric value.");
            return value;
        }
        let funcDefinition: ASTNode | null = null;

        if (currentInstance && funcName in currentInstance.methods) {
            funcDefinition = currentInstance.methods[funcName];
        } else if (funcName in this.functions) {
            funcDefinition = this.functions[funcName];
        } else {
            throw new Error(`Runtime Error: Call to undefined function or method '${funcName}'`);
        }

        const expectedParams = funcDefinition!.parameters as string[];
        const isMethod = funcDefinition!.is_method;
        const paramOffset = (isMethod && expectedParams[0] === 'self') ? 1 : 0;
        const expectedArgsCount = expectedParams.length - paramOffset;

        if (expectedArgsCount !== providedArgsNodes.length) {
            throw new Error(`Runtime Error: Function/Method '${funcName}' expected ${expectedArgsCount} arguments but got ${providedArgsNodes.length}`);
        }

        this.push_scope();

        // 1. Bind 'self' for methods
        if (isMethod && currentInstance) {
            this.setVariable('self', currentInstance, 'LET');
        }

        // 2. Evaluate arguments
        const evaluatedArgs = providedArgsNodes.map(argNode => this.visit(argNode, currentInstance));

        // 3. Bind arguments to parameters
        for (let i = 0; i < expectedArgsCount; i++) {
            this.setVariable(expectedParams[i + paramOffset], evaluatedArgs[i], 'LET');
        }

        let returnValue: any = null;
        try {
            for (const statement of funcDefinition!.body) {
                this.visit(statement, currentInstance);
            }
        } catch (e) {
            if (e instanceof FunctionReturn) {
                returnValue = e.value;
            } else {
                throw e;
            }
        } finally {
            this.pop_scope();
        }
        return returnValue;
    }
    
    private visitReturnStatement(node: ASTNode): never {
        const returnValue = this.visit(node.expression);
        throw new FunctionReturn(returnValue);
    }
    
    private visitClassDefinition(node: ASTNode): void {
        const className = node.name;
        if (className in this.user_defined_classes) {
            throw new Error(`Runtime Error: Class '${className}' already defined.`);
        }
        this.user_defined_classes[className] = node;
    }

    private visitObjectCreation(node: ASTNode): PayJarObject {
        const className = node.class_name;
        const evaluatedArgs = node.arguments.map((arg: ASTNode) => this.visit(arg));

        const classDefinition = this.user_defined_classes[className];
        if (!classDefinition) {
            throw new Error(`Runtime Error: Attempt to create instance of undefined class '${className}'`);
        }

        const instanceFields: any = {};
        const instanceMethods: any = {};

        // Initialize fields
        for (const field of classDefinition.fields) {
            const initialValue = field.value ? this.visit(field.value) : null;
            instanceFields[field.name] = { value: initialValue, kind: field.kind };
        }

        // Map methods
        for (const methodNode of classDefinition.methods) {
            (methodNode as any).is_method = true;
            instanceMethods[methodNode.name] = methodNode;
        }

        const instance = new PayJarObject(className, instanceFields, instanceMethods);

        // Execute constructor (init method)
        if (classDefinition.constructor) {
            const constructorNode = classDefinition.constructor as any;
            constructorNode.is_method = true; 
            const expectedConstructorParams = constructorNode.parameters as string[];
            
            // Check for 'self' parameter
            if (!expectedConstructorParams || expectedConstructorParams[0] !== 'self') {
                 throw new Error(`Runtime Error: Constructor 'init' for class '${className}' must have 'self' as its first parameter.`);
            }
            
            if ((expectedConstructorParams.length - 1) !== evaluatedArgs.length) {
                throw new Error(`Runtime Error: Constructor for '${className}' expected ${expectedConstructorParams.length - 1} arguments but got ${evaluatedArgs.length}`);
            }

            this.push_scope();
            this.setVariable('self', instance, 'LET');
            
            for (let i = 0; i < evaluatedArgs.length; i++) {
                this.setVariable(expectedConstructorParams[i + 1], evaluatedArgs[i], 'LET');
            }

            try {
                for (const statement of constructorNode.body) {
                    this.visit(statement, instance);
                }
            } catch (e) {
                if (!(e instanceof FunctionReturn)) throw e;
            } finally {
                this.pop_scope();
            }
        }
        return instance;
    }

    private visitMemberAccess(node: ASTNode, currentInstance: PayJarObject | null): any {
        const obj = this.visit(node.object);
        if (!(obj instanceof PayJarObject)) {
            throw new Error(`Runtime Error: Attempt to access member '${node.member}' on a non-object type.`);
        }

        const memberName = node.member;
        
        if (node.is_call) { // Method call
            if (!(memberName in obj.methods)) {
                throw new Error(`Runtime Error: Method '${memberName}' not found on object of type '${obj.class_name}'`);
            }
            // Delegate to function call logic, passing the object itself as the instance
            return this.visitFunctionCall({ type: "function_call", name: memberName, arguments: node.arguments }, obj);
        } else { // Field access
            if (!(memberName in obj.fields)) {
                throw new Error(`Runtime Error: Field '${memberName}' not found on object of type '${obj.class_name}'`);
            }
            return obj.fields[memberName].value;
        }
    }

    private visitMemberAssignment(node: ASTNode, currentInstance: PayJarObject | null): void {
        const obj = this.visit(node.object);
        if (!(obj instanceof PayJarObject)) {
            throw new Error(`Runtime Error: Attempt to assign member '${node.member}' on a non-object type.`);
        }
        const memberName = node.member;
        const newValue = this.visit(node.value);

        if (!(memberName in obj.fields)) {
            throw new Error(`Runtime Error: Field '${memberName}' not found on object of type '${obj.class_name}' for assignment.`);
        }
        if (obj.fields[memberName].kind === 'CONST') {
            throw new Error(`Runtime Error: Cannot assign to constant field '${memberName}' of object '${obj.class_name}'.`);
        }
        obj.fields[memberName].value = newValue;
    }

    private visitBinaryOp(node: ASTNode): any {
        const leftVal = this.visit(node.left);
        const rightVal = this.visit(node.right);
        const operator = node.operator;

        if (typeof leftVal !== typeof rightVal) {
             // Basic type compatibility check, often required for concatenation vs arithmetic
        }

        switch (operator) {
            case '+': return leftVal + rightVal;
            case '-': return leftVal - rightVal;
            case '*': return leftVal * rightVal;
            case '/':
                if (rightVal === 0) throw new Error("Runtime Error: Division by zero.");
                // Payjar 1.5 always produces a Float for ordinary division (e.g. 1 / 2 == 0.5).
                return Number(leftVal) / Number(rightVal);
            case '%': {
                const dividend = Math.trunc(Number(leftVal));
                const divisor = Math.trunc(Number(rightVal));
                if (divisor === 0) throw new Error("Runtime Error: Modulo by zero.");
                return dividend % divisor;
            }
            // The native runtime represents boolean results as integer 1/0.
            case '==': return leftVal === rightVal ? 1 : 0;
            case '!=': return leftVal !== rightVal ? 1 : 0;
            case '<': return leftVal < rightVal ? 1 : 0;
            case '>': return leftVal > rightVal ? 1 : 0;
            case '<=': return leftVal <= rightVal ? 1 : 0;
            case '>=': return leftVal >= rightVal ? 1 : 0;
            case 'and': case '&&': return Boolean(leftVal) && Boolean(rightVal) ? 1 : 0;
            case 'or': case '||': return Boolean(leftVal) || Boolean(rightVal) ? 1 : 0;
            default: throw new Error(`Runtime Error: Unsupported binary operator: ${operator}`);
        }
    }

    private visitUnaryOp(node: ASTNode): any {
        const operandVal = this.visit(node.operand);
        const operator = node.operator;
        if (operator === '!' || operator === 'not') return Boolean(operandVal) ? 0 : 1;
        if (typeof operandVal !== 'number') throw new Error(`Runtime Error: Unary operator ${operator} applied to non-numeric type.`);
        if (operator === '+') return +operandVal;
        if (operator === '-') return -operandVal;
        throw new Error(`Runtime Error: Unsupported unary operator: ${operator}`);
    }
}

// ====================================================================
// 6. Runtime Execution Classes (PJRT, PJS)
// ====================================================================

// PJRT (PayJar RunTime)
export class PJRT {
    constructor(
        private code: string,
        private debug: boolean = false,
        private inputProvider?: (prompt?: string) => string,
        private workspaceFiles: Record<string, string> = {},
        private entryPath: string = 'main.pj'
    ) {}

    private resolveImports(source: string, fromPath: string, stack: string[] = []): string {
        const directory = fromPath.includes('/') ? fromPath.slice(0, fromPath.lastIndexOf('/') + 1) : '';
        const importPattern = /^\s*import\s+(?:"([^"]+)"|'([^']+)'|([A-Za-z0-9_./-]+))\s*;?\s*$/gm;
        let result = '';
        let cursor = 0;
        let match: RegExpExecArray | null;
        while ((match = importPattern.exec(source)) !== null) {
            result += source.slice(cursor, match.index);
            cursor = match.index + match[0].length;
            const requested = (match[1] || match[2] || match[3] || '').replace(/\\/g, '/');
            const combined = requested.startsWith('/') ? requested.slice(1) : directory + requested;
            const parts: string[] = [];
            for (const part of combined.split('/')) {
                if (!part || part === '.') continue;
                if (part === '..') {
                    if (!parts.length) throw new Error(`Import Error: '${requested}' escapes the workspace root.`);
                    parts.pop();
                } else parts.push(part);
            }
            let resolved = parts.join('/');
            if (!/\.[A-Za-z0-9]+$/.test(resolved)) resolved += '.pj';
            if (stack.includes(resolved)) throw new Error(`Import Error: Circular import detected: ${[...stack, resolved].join(' -> ')}`);
            const imported = this.workspaceFiles[resolved];
            if (typeof imported !== 'string') throw new Error(`Import Error: Cannot find '${resolved}' in the IDE workspace. Add the file to the Explorer first.`);
            result += this.resolveImports(imported, resolved, [...stack, resolved]) + '\n';
        }
        return result + source.slice(cursor);
    }

    public run_code(debug_on: boolean, inputProvider?: (prompt?: string) => string): { tokens: Token[], ast: MainDefinitionNode | null, outputs: string[] } {
        let tokens: Token[] = [];
        let ast: MainDefinitionNode | null = null;
        let outputs: string[] = [];
        try {
            const source = this.resolveImports(this.code, this.entryPath, [this.entryPath]);
            const lexer = new Lexer(source);
            tokens = lexer.tokenize();
            if (debug_on) console.log("Lexer Tokens:", tokens.map(t => t.toString()));

            const parser = new Parser(tokens);
            ast = parser.parse() as MainDefinitionNode;
            if (debug_on) console.log("Parsed AST:", JSON.stringify(ast, null, 2));

            const interpreter = new Interpreter(inputProvider ?? this.inputProvider);
            interpreter.interpret(ast);
            outputs = interpreter.getOutputs();
            
        } catch (e) {
            throw e;
        }
        return { tokens, ast, outputs };
    }
}

// PJS (PayJar Shell/Simulator - Highly simplified, as the Python version seemed incomplete)
export class PJS {
    // In a real TS environment, PJS would handle the input/output shell.
    // Given the Python PJS was mostly a wrapper for PJRT, we'll keep it simple.
    
    // The original Python PJS seemed to confuse its own AST with the COMP AST.
    // For TypeScript, we'll just demonstrate calling PJRT.
    public run(code: string, inputData: string | ((prompt?: string) => string) = ""): void {
        console.log(`--- Running Code (Input Simulated) ---`);
        // Build an input provider based on inputData
        let provider: ((prompt?: string) => string) | undefined = undefined;
        if (typeof inputData === 'string') {
            // Return the same input string for all prompts (simple simulation)
            let used = false;
            provider = (_prompt?: string) => {
                // on subsequent calls, return empty string
                if (!used) { used = true; return inputData; }
                return "";
            };
        } else if (typeof inputData === 'function') {
            provider = inputData;
        }

        // Use the PJRT with the provider so READLN can be satisfied
        new PJRT(code, true, provider).run_code(true, provider);
        console.log(`--- Execution Complete ---`);
    }
}

// Example usage if this file were run directly (Node.js environment):
/*
const sampleCode = `
public class Main(@self) {
    let x = 10;
    println(x + 5);
}
`;

new PJRT(sampleCode, true);
*/
