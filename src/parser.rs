use crate::ast::*;
use crate::lexer::Token;

pub struct Parser {
    tokens: Vec<Token>,
    current: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Parser { tokens, current: 0 }
    }

    fn current_token(&self) -> &Token {
        self.tokens.get(self.current).unwrap_or(&Token::Eof)
    }

    fn advance(&mut self) -> &Token {
        if self.current < self.tokens.len() {
            self.current = self.current + 1;
        }
        self.current_token()
    }

    fn expect(&mut self, expected: Token) -> Result<(), String> {
        if std::mem::discriminant(self.current_token()) == std::mem::discriminant(&expected) {
            self.advance();
            Ok(())
        } else {
            Err(format!("Expected {:?}, found {:?}", expected, self.current_token()))
        }
    }

    fn skip_newlines(&mut self) {
        while matches!(self.current_token(), Token::Newline) {
            self.advance();
        }
    }

    pub fn parse(&mut self) -> Result<Program, String> {
        let mut statements = Vec::new();
        self.skip_newlines();

        while !matches!(self.current_token(), Token::Eof) {
            statements.push(self.parse_statement()?);
            self.skip_newlines();
        }

        Ok(Program { statements })
    }

    fn parse_statement(&mut self) -> Result<Statement, String> {
        match self.current_token() {
            Token::Let => self.parse_let_statement(),
            Token::Show => self.parse_show_statement(),
            Token::When => self.parse_when_statement(),
            Token::Define => self.parse_function_def(),
            _ => {
                let expr = self.parse_expression()?;
                Ok(Statement::Expression(expr))
            }
        }
    }

    fn parse_let_statement(&mut self) -> Result<Statement, String> {
        self.expect(Token::Let)?;
        
        let identifier = match self.current_token() {
            Token::Identifier(name) => {
                let name = name.clone();
                self.advance();
                name
            }
            _ => return Err("Expected identifier after 'let'".to_string()),
        };
        
        self.expect(Token::Be)?;
        
        let value = self.parse_expression()?;
        
        Ok(Statement::Let(LetStatement { identifier, value }))
    }

    fn parse_show_statement(&mut self) -> Result<Statement, String> {
        self.expect(Token::Show)?;
        let value = self.parse_expression()?;
        Ok(Statement::Show(ShowStatement { value }))
    }
    
    fn parse_when_statement(&mut self) -> Result<Statement, String> {
        self.expect(Token::When)?;

        let condition = self.parse_expression()?;

        self.expect(Token::Then)?;
        self.skip_newlines();

        // For Indented Block(s): FUCKKKK
        let mut then_block = Vec::new();
        if matches!(self.current_token(), Token::Indent) {
            self.advance(); // Go Over Indent

            while !matches!(self.current_token(), Token::Dedent | Token::Otherwise | Token::Eof) {
                then_block.push(self.parse_statement()?);
                self.skip_newlines();
            }

            if matches!(self.current_token(), Token::Dedent) {
                self.advance(); // Go Over Dedent
            }
        }

        let otherwise_block = if matches!(self.current_token(), Token::Otherwise) {
            self.advance(); // Go Over Otherwise
            self.skip_newlines();

            let mut otherwise_statements = Vec::new();
            if matches!(self.current_token(), Token::Indent) {
                self.advance(); // Go Over Indent

                while !matches!(self.current_token(), Token::Dedent | Token::Eof) {
                    otherwise_statements.push(self.parse_statement()?);
                    self.skip_newlines();
                }

                if matches!(self.current_token(), Token::Dedent) {
                    self.advance();
                }
            }

            Some(otherwise_statements)
        } else {
            None
        };

                
        Ok(Statement::When(WhenStatement {
            condition,
            then_block,
            otherwise_block,
        }))
    }

    fn parse_function_def(&mut self) -> Result<Statement, String> {
        self.expect(Token::Define)?;
        
        let name = match self.current_token() {
            Token::Identifier(name) => {
                let name = name.clone();
                self.advance();
                name
            }
            _ => return Err("Expected function name after 'define'".to_string()),
        };
        
        let mut parameters = Vec::new();
        
        if matches!(self.current_token(), Token::With) {
            self.advance(); // consume 'with'
            
            loop {
                match self.current_token() {
                    Token::Identifier(param) => {
                        parameters.push(param.clone());
                        self.advance();
                    }
                    _ => break,
                }
                
                // Check for more parameters (basic implementation)
                if !matches!(self.current_token(), Token::Identifier(_)) {
                    break;
                }
            }
        }
        
        self.skip_newlines();
        
        let mut body = Vec::new();
        if matches!(self.current_token(), Token::Indent) {
            self.advance(); // Go Over Indent
            
            while !matches!(self.current_token(), Token::End | Token::Dedent | Token::Eof) {
                body.push(self.parse_statement()?);
                self.skip_newlines();
            }
            
            // Handle either 'end' keyword or dedent
            if matches!(self.current_token(), Token::Dedent) {
                self.advance();
            }
        }
        
        if matches!(self.current_token(), Token::End) {
            self.advance();
        }
        
        Ok(Statement::FunctionDef(FunctionDef {
            name,
            parameters,
            body,
        }))
    }
    
    fn parse_expression(&mut self) -> Result<Expression, String> {
        self.parse_comparison()
    }
    
    fn parse_comparison(&mut self) -> Result<Expression, String> {
        let mut left = self.parse_arithmetic()?;
        
        while let Some(op) = self.parse_comparison_operator() {
            let right = self.parse_arithmetic()?;
            left = Expression::BinaryOp(BinaryOperation {
                left: Box::new(left),
                operator: op,
                right: Box::new(right),
            });
        }
        
        Ok(left)
    }

    fn parse_arithmetic(&mut self) -> Result<Expression, String> {
        let mut left = self.parse_term()?;
        
        while matches!(self.current_token(), Token::Plus | Token::Minus) {
            let op = match self.current_token() {
                Token::Plus => {
                    self.advance();
                    BinaryOperator::Add
                }
                Token::Minus => {
                    self.advance();
                    BinaryOperator::Subtract
                }
                _ => break,
            };
            
            let right = self.parse_term()?;
            left = Expression::BinaryOp(BinaryOperation {
                left: Box::new(left),
                operator: op,
                right: Box::new(right),
            });
        }
        
        Ok(left)
    }
    
    fn parse_term(&mut self) -> Result<Expression, String> {
        let mut left = self.parse_primary()?;
        
        while matches!(self.current_token(), Token::Multiply | Token::Divide) {
            let op = match self.current_token() {
                Token::Multiply => {
                    self.advance();
                    BinaryOperator::Multiply
                }
                Token::Divide => {
                    self.advance();
                    BinaryOperator::Divide
                }
                _ => break,
            };
            
            let right = self.parse_primary()?;
            left = Expression::BinaryOp(BinaryOperation {
                left: Box::new(left),
                operator: op,
                right: Box::new(right),
            });
        }
        
        Ok(left)
    }

    fn parse_comparison_operator(&mut self) -> Option<BinaryOperator> {
        match self.current_token() {
            Token::IsGreaterThan => {
                self.advance();
                Some(BinaryOperator::GreaterThan)
            }
            Token::IsLessThan => {
                self.advance();
                Some(BinaryOperator::LessThan)
            }
            Token::IsGreaterThanOrEqual => {
                self.advance();
                Some(BinaryOperator::GreaterThanOrEqual)
            }
            Token::IsLessThanOrEqual => {
                self.advance();
                Some(BinaryOperator::LessThanOrEqual)
            }
            Token::IsEqual => {
                self.advance();
                Some(BinaryOperator::Equal)
            }
            Token::IsNotEqual => {
                self.advance();
                Some(BinaryOperator::NotEqual)
            }
            _ => None,
        }
    }

    // The `parse_primary` function is Generated by AI.
    fn parse_primary(&mut self) -> Result<Expression, String> {
        match self.current_token().clone() {
            Token::Number(n) => {
                self.advance();
                Ok(Expression::Number(n))
            }
            Token::String(s) => {
                self.advance();
                Ok(Expression::String(s))
            }
            Token::Identifier(name) => {
                self.advance();
                // Check if this is a function call (basic implementation)
                Ok(Expression::Identifier(name))
            }
            _ => Err(format!("Unexpected token in expression: {:?}", self.current_token())),
        }
    }
}