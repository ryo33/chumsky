use super::*;

/// See [`Parser::ignored`].
pub type Ignored<P, O> = To<P, O, ()>;

/// See [`Parser::ignore_then`].
pub type IgnoreThen<A, B, O, U> = Map<Then<A, B>, fn((O, U)) -> U, (O, U)>;

/// See [`Parser::then_ignore`].
pub type ThenIgnore<A, B, O, U> = Map<Then<A, B>, fn((O, U)) -> O, (O, U)>;

/// See [`Parser::or`].
#[derive(Copy, Clone)]
pub struct Or<A, B>(pub(crate) A, pub(crate) B);

impl<I: Clone, O, A: Parser<I, O, Error = E>, B: Parser<I, O, Error = E>, E: Error<I>> Parser<I, O>
    for Or<A, B>
{
    type Error = E;

    #[inline]
    fn parse_inner<D: Debugger>(
        &self,
        debugger: &mut D,
        stream: &mut StreamOf<I, E>,
    ) -> PResult<I, O, E> {
        let pre_state = stream.save();

        #[allow(deprecated)]
        let a_res = debugger.invoke(&self.0, stream);
        let a_state = stream.save();

        // If the first parser succeeded and produced no secondary errors, don't bother trying the second parser
        if a_res.0.len() == 0 {
            if let (a_errors, Ok(a_out)) = a_res {
                return (a_errors, Ok(a_out));
            }
        }

        stream.revert(pre_state);

        #[allow(deprecated)]
        let b_res = debugger.invoke(&self.1, stream);
        let b_state = stream.save();

        fn zip_with<A, B, R, F: FnOnce(A, B) -> R>(a: Option<A>, b: Option<B>, f: F) -> Option<R> {
            match (a, b) {
                (Some(a), Some(b)) => Some(f(a, b)),
                _ => None,
            }
        }

        let is_a = match (&a_res, &b_res) {
            ((a_errors, Ok(a_out)), (b_errors, Ok(b_out))) => {
                match a_errors.len().cmp(&b_errors.len()) {
                    Ordering::Greater => false,
                    Ordering::Less => true,
                    Ordering::Equal => {
                        match zip_with(a_errors.last(), b_errors.last(), |a, b| a.at.cmp(&b.at)) {
                            Some(Ordering::Greater) => true,
                            Some(Ordering::Less) => false,
                            _ => match zip_with(a_out.1.as_ref(), b_out.1.as_ref(), |a, b| {
                                a.at.cmp(&b.at)
                            }) {
                                Some(Ordering::Greater) => true,
                                Some(Ordering::Less) => false,
                                _ => true,
                            },
                        }
                    }
                }
            }
            // ((a_errors, Ok(_)), (b_errors, Err(_))) if !a_errors.is_empty() => panic!("a_errors = {:?}", a_errors.iter().map(|e| e.debug()).collect::<Vec<_>>()),
            ((_a_errors, Ok(_)), (_b_errors, Err(_))) => true,
            // ((a_errors, Err(_)), (b_errors, Ok(_))) if !b_errors.is_empty() => panic!("b_errors = {:?}", b_errors.iter().map(|e| e.debug()).collect::<Vec<_>>()),
            ((_a_errors, Err(_)), (_b_errors, Ok(_))) => false,
            ((a_errors, Err(a_err)), (b_errors, Err(b_err))) => match a_err.at.cmp(&b_err.at) {
                Ordering::Greater => true,
                Ordering::Less => false,
                Ordering::Equal => match a_errors.len().cmp(&b_errors.len()) {
                    Ordering::Greater => false,
                    Ordering::Less => true,
                    Ordering::Equal => {
                        match zip_with(a_errors.last(), b_errors.last(), |a, b| a.at.cmp(&b.at)) {
                            Some(Ordering::Greater) => true,
                            Some(Ordering::Less) => false,
                            // If the branches really do seem to be equally valid as parse options, try to unify them
                            // We already know that both parsers produces hard errors, so unwrapping cannot fail here
                            _ => {
                                return (
                                    a_res.0,
                                    Err(a_res.1.err().unwrap().max(b_res.1.err().unwrap())),
                                )
                            }
                        }
                    }
                },
            },
        };

        if is_a {
            stream.revert(a_state);
            (
                a_res.0,
                a_res.1.map(|(out, alt)| {
                    (
                        out,
                        merge_alts(alt, b_res.1.map(|(_, alt)| alt).unwrap_or_else(|e| Some(e))),
                    )
                }),
            )
        } else {
            stream.revert(b_state);
            (
                b_res.0,
                b_res.1.map(|(out, alt)| {
                    (
                        out,
                        merge_alts(alt, a_res.1.map(|(_, alt)| alt).unwrap_or_else(|e| Some(e))),
                    )
                }),
            )
        }
    }

    #[inline]
    fn parse_inner_verbose(&self, d: &mut Verbose, s: &mut StreamOf<I, E>) -> PResult<I, O, E> {
        #[allow(deprecated)]
        self.parse_inner(d, s)
    }
    #[inline]
    fn parse_inner_silent(&self, d: &mut Silent, s: &mut StreamOf<I, E>) -> PResult<I, O, E> {
        #[allow(deprecated)]
        self.parse_inner(d, s)
    }
}

/// See [`Parser::or_not`].
#[derive(Copy, Clone)]
pub struct OrNot<A>(pub(crate) A);

impl<I: Clone, O, A: Parser<I, O, Error = E>, E: Error<I>> Parser<I, Option<O>> for OrNot<A> {
    type Error = E;

    #[inline]
    fn parse_inner<D: Debugger>(
        &self,
        debugger: &mut D,
        stream: &mut StreamOf<I, E>,
    ) -> PResult<I, Option<O>, E> {
        match stream.try_parse(|stream| {
            #[allow(deprecated)]
            debugger.invoke(&self.0, stream)
        }) {
            (errors, Ok((out, alt))) => (errors, Ok((Some(out), alt))),
            (_, Err(err)) => (Vec::new(), Ok((None, Some(err)))),
        }
    }

    #[inline]
    fn parse_inner_verbose(
        &self,
        d: &mut Verbose,
        s: &mut StreamOf<I, E>,
    ) -> PResult<I, Option<O>, E> {
        #[allow(deprecated)]
        self.parse_inner(d, s)
    }
    #[inline]
    fn parse_inner_silent(
        &self,
        d: &mut Silent,
        s: &mut StreamOf<I, E>,
    ) -> PResult<I, Option<O>, E> {
        #[allow(deprecated)]
        self.parse_inner(d, s)
    }
}

/// See [`Parser::then`].
#[derive(Copy, Clone)]
pub struct Then<A, B>(pub(crate) A, pub(crate) B);

impl<I: Clone, O, U, A: Parser<I, O, Error = E>, B: Parser<I, U, Error = E>, E: Error<I>>
    Parser<I, (O, U)> for Then<A, B>
{
    type Error = E;

    #[inline]
    fn parse_inner<D: Debugger>(
        &self,
        debugger: &mut D,
        stream: &mut StreamOf<I, E>,
    ) -> PResult<I, (O, U), E> {
        match {
            #[allow(deprecated)]
            debugger.invoke(&self.0, stream)
        } {
            (mut a_errors, Ok((a_out, a_alt))) => match {
                #[allow(deprecated)]
                debugger.invoke(&self.1, stream)
            } {
                (mut b_errors, Ok((b_out, b_alt))) => {
                    a_errors.append(&mut b_errors);
                    (a_errors, Ok(((a_out, b_out), merge_alts(a_alt, b_alt))))
                }
                (mut b_errors, Err(b_err)) => {
                    a_errors.append(&mut b_errors);
                    (a_errors, Err(b_err.max(a_alt)))
                }
            },
            (a_errors, Err(a_err)) => (a_errors, Err(a_err)),
        }
    }

    #[inline]
    fn parse_inner_verbose(
        &self,
        d: &mut Verbose,
        s: &mut StreamOf<I, E>,
    ) -> PResult<I, (O, U), E> {
        #[allow(deprecated)]
        self.parse_inner(d, s)
    }
    #[inline]
    fn parse_inner_silent(&self, d: &mut Silent, s: &mut StreamOf<I, E>) -> PResult<I, (O, U), E> {
        #[allow(deprecated)]
        self.parse_inner(d, s)
    }
}

/// See [`Parser::delimited_by`].
#[derive(Copy, Clone)]
pub struct DelimitedBy<A, I>(pub(crate) A, pub(crate) I, pub(crate) I);

impl<I: Clone + PartialEq, O, A: Parser<I, O, Error = E>, E: Error<I>> Parser<I, O>
    for DelimitedBy<A, I>
{
    type Error = E;

    #[inline]
    fn parse_inner<D: Debugger>(
        &self,
        debugger: &mut D,
        stream: &mut StreamOf<I, E>,
    ) -> PResult<I, O, E> {
        // TODO: Don't clone!
        #[allow(deprecated)]
        let (errors, res) = debugger.invoke(
            &just(self.1.clone())
                .ignore_then(&self.0)
                .then_ignore(just(self.2.clone())),
            stream,
        );
        (errors, res)
    }

    #[inline]
    fn parse_inner_verbose(&self, d: &mut Verbose, s: &mut StreamOf<I, E>) -> PResult<I, O, E> {
        #[allow(deprecated)]
        self.parse_inner(d, s)
    }
    #[inline]
    fn parse_inner_silent(&self, d: &mut Silent, s: &mut StreamOf<I, E>) -> PResult<I, O, E> {
        #[allow(deprecated)]
        self.parse_inner(d, s)
    }
}

/// See [`Parser::repeated`].
#[derive(Copy, Clone)]
pub struct Repeated<A>(pub(crate) A, pub(crate) usize, pub(crate) Option<usize>);

impl<A> Repeated<A> {
    /// Require that the pattern appear at least a minimum number of times.
    pub fn at_least(mut self, min: usize) -> Self {
        self.1 = min;
        self
    }

    /// Require that the pattern appear at most a maximum number of times.
    pub fn at_most(mut self, max: usize) -> Self {
        self.2 = Some(max);
        self
    }
}

impl<I: Clone, O, A: Parser<I, O, Error = E>, E: Error<I>> Parser<I, Vec<O>> for Repeated<A> {
    type Error = E;

    #[inline]
    fn parse_inner<D: Debugger>(
        &self,
        debugger: &mut D,
        stream: &mut StreamOf<I, E>,
    ) -> PResult<I, Vec<O>, E> {
        let mut errors = Vec::new();
        let mut outputs = Vec::new();
        let mut alt = None;
        let mut old_offset = None;

        loop {
            if self.2.map_or(false, |max| outputs.len() >= max) {
                break (errors, Ok((outputs, alt)));
            }

            if let ControlFlow::Break(b) = stream.attempt(|stream| match { #[allow(deprecated)] debugger.invoke(&self.0, stream) } {
                (mut a_errors, Ok((a_out, a_alt))) => {
                    errors.append(&mut a_errors);
                    alt = merge_alts(alt.take(), a_alt);
                    outputs.push(a_out);

                    if old_offset == Some(stream.offset()) {
                        panic!("Repeated parser iteration succeeded but consumed no inputs (i.e: continuing \
                            iteration would likely lead to an infinite loop, if the parser is pure). This is \
                            likely indicative of a parser bug. Consider using a more specific error recovery \
                            strategy.");
                    } else {
                        old_offset = Some(stream.offset());
                    }

                    (true, ControlFlow::Continue(()))
                },
                (mut a_errors, Err(a_err)) if outputs.len() < self.1 => {
                    errors.append(&mut a_errors);
                    (true, ControlFlow::Break((
                        std::mem::take(&mut errors),
                        Err(a_err),
                    )))
                },
                (a_errors, Err(a_err)) => {
                    // Find furthest alternative error
                    // TODO: Handle multiple alternative errors
                    // TODO: Should we really be taking *all* of these into consideration?
                    let alt = merge_alts(
                        alt.take(),
                        merge_alts(
                            Some(a_err),
                            a_errors.into_iter().next(),
                        ),
                    );
                    (false, ControlFlow::Break((
                        std::mem::take(&mut errors),
                        Ok((std::mem::take(&mut outputs), alt)),
                    )))
                },
            }) {
                break b;
            }
        }
    }

    #[inline]
    fn parse_inner_verbose(
        &self,
        d: &mut Verbose,
        s: &mut StreamOf<I, E>,
    ) -> PResult<I, Vec<O>, E> {
        #[allow(deprecated)]
        self.parse_inner(d, s)
    }
    #[inline]
    fn parse_inner_silent(&self, d: &mut Silent, s: &mut StreamOf<I, E>) -> PResult<I, Vec<O>, E> {
        #[allow(deprecated)]
        self.parse_inner(d, s)
    }
}

/// See [`Parser::separated_by`].
pub struct SeparatedBy<A, B, U> {
    pub(crate) item: A,
    pub(crate) delimiter: B,
    pub(crate) at_least: usize,
    pub(crate) allow_leading: bool,
    pub(crate) allow_trailing: bool,
    pub(crate) phantom: PhantomData<U>,
}

impl<A, B, U> SeparatedBy<A, B, U> {
    /// Allow a leading separator to appear before the first item.
    ///
    /// # Examples
    ///
    /// ```
    /// # use chumsky::prelude::*;
    /// let r#enum = seq::<_, _, Simple<char>>("enum".chars())
    ///     .padded()
    ///     .ignore_then(text::ident()
    ///         .padded()
    ///         .separated_by(just('|'))
    ///         .allow_leading());
    ///
    /// assert_eq!(r#enum.parse("enum True | False"), Ok(vec!["True".to_string(), "False".to_string()]));
    /// assert_eq!(r#enum.parse("
    ///     enum
    ///     | True
    ///     | False
    /// "), Ok(vec!["True".to_string(), "False".to_string()]));
    /// ```
    pub fn allow_leading(mut self) -> Self {
        self.allow_leading = true;
        self
    }

    /// Allow a trailing separator to appear after the last item.
    ///
    /// # Examples
    ///
    /// ```
    /// # use chumsky::prelude::*;
    /// let numbers = text::int::<_, Simple<char>>(10)
    ///     .padded()
    ///     .separated_by(just(','))
    ///     .allow_trailing()
    ///     .delimited_by('(', ')');
    ///
    /// assert_eq!(numbers.parse("(1, 2)"), Ok(vec!["1".to_string(), "2".to_string()]));
    /// assert_eq!(numbers.parse("(1, 2,)"), Ok(vec!["1".to_string(), "2".to_string()]));
    /// ```
    pub fn allow_trailing(mut self) -> Self {
        self.allow_trailing = true;
        self
    }

    /// Require that the pattern appear at least a minimum number of times.
    ///
    /// ```
    /// # use chumsky::prelude::*;
    /// let numbers = just::<_, Simple<char>>('-')
    ///     .separated_by(just('.'))
    ///     .at_least(2);
    ///
    /// assert!(numbers.parse("").is_err());
    /// assert!(numbers.parse("-").is_err());
    /// assert_eq!(numbers.parse("-.-"), Ok(vec!['-', '-']));
    /// ````
    pub fn at_least(mut self, n: usize) -> Self {
        self.at_least = n;
        self
    }
}

impl<A: Copy, B: Copy, U> Copy for SeparatedBy<A, B, U> {}
impl<A: Clone, B: Clone, U> Clone for SeparatedBy<A, B, U> {
    fn clone(&self) -> Self {
        Self {
            item: self.item.clone(),
            delimiter: self.delimiter.clone(),
            at_least: self.at_least,
            allow_leading: self.allow_leading,
            allow_trailing: self.allow_trailing,
            phantom: PhantomData,
        }
    }
}

impl<I: Clone, O, U, A: Parser<I, O, Error = E>, B: Parser<I, U, Error = E>, E: Error<I>>
    Parser<I, Vec<O>> for SeparatedBy<A, B, U>
{
    type Error = E;

    #[inline]
    fn parse_inner<D: Debugger>(
        &self,
        debugger: &mut D,
        stream: &mut StreamOf<I, E>,
    ) -> PResult<I, Vec<O>, E> {
        enum State<I, E> {
            Terminated(Located<I, E>),
            Continue,
        }

        fn parse_or_not<U, B: Parser<I, U, Error = E>, I: Clone, E: Error<I>, D: Debugger>(
            delimiter: &B,
            stream: &mut StreamOf<I, E>,
            debugger: &mut D,
            alt: Option<Located<I, E>>,
        ) -> Option<Located<I, E>> {
            match stream.try_parse(|stream| {
                #[allow(deprecated)]
                debugger.invoke(&delimiter, stream)
            }) {
                // These two paths are successful path so the furthest errors are merged with the alt.
                (d_errors, Ok((_, d_alt))) => merge_alts(alt, merge_alts(d_alt, d_errors)),
                (d_errors, Err(d_err)) => merge_alts(alt, merge_alts(Some(d_err), d_errors)),
            }
        }

        fn parse<O, A: Parser<I, O, Error = E>, I: Clone, E: Error<I>, D: Debugger>(
            item: &A,
            stream: &mut StreamOf<I, E>,
            debugger: &mut D,
            outputs: &mut Vec<O>,
            errors: &mut Vec<Located<I, E>>,
            alt: Option<Located<I, E>>,
        ) -> (State<I, E>, Option<Located<I, E>>) {
            match stream.try_parse(|stream| {
                #[allow(deprecated)]
                debugger.invoke(item, stream)
            }) {
                (mut i_errors, Ok((i_out, i_alt))) => {
                    outputs.push(i_out);
                    errors.append(&mut i_errors);
                    (State::Continue, merge_alts(alt, i_alt))
                }
                (mut i_errors, Err(i_err)) => {
                    errors.append(&mut i_errors);
                    (State::Terminated(i_err), alt)
                }
            }
        }

        let mut outputs = Vec::new();
        let mut errors = Vec::new();
        let mut alt = None;

        if self.allow_leading {
            alt = parse_or_not(&self.delimiter, stream, debugger, alt);
        }

        let (mut state, mut alt) =
            parse(&self.item, stream, debugger, &mut outputs, &mut errors, alt);

        let mut offset = stream.save();
        let error: Located<I, E>;
        loop {
            if let State::Terminated(err) = state {
                error = err;
                break;
            }
            offset = stream.save();

            match stream.try_parse(|stream| {
                #[allow(deprecated)]
                debugger.invoke(&self.delimiter, stream)
            }) {
                (mut d_errors, Ok((_, d_alt))) => {
                    errors.append(&mut d_errors);
                    alt = merge_alts(alt, d_alt);

                    let (i_state, i_alt) =
                        parse(&self.item, stream, debugger, &mut outputs, &mut errors, alt);
                    state = i_state;
                    alt = i_alt;
                }
                (mut d_errors, Err(d_err)) => {
                    errors.append(&mut d_errors);
                    state = State::Terminated(d_err);
                }
            }
        }
        stream.revert(offset);

        if self.allow_trailing {
            alt = parse_or_not(&self.delimiter, stream, debugger, alt);
        }

        if outputs.len() >= self.at_least {
            alt = merge_alts(alt, Some(error));
            (errors, Ok((outputs, alt)))
        } else {
            // In all paths where `State = State::Terminated`, Some(err) is inserted into alt.
            (errors, Err(error))
        }
    }

    #[inline]
    fn parse_inner_verbose(
        &self,
        d: &mut Verbose,
        s: &mut StreamOf<I, E>,
    ) -> PResult<I, Vec<O>, E> {
        #[allow(deprecated)]
        self.parse_inner(d, s)
    }
    #[inline]
    fn parse_inner_silent(&self, d: &mut Silent, s: &mut StreamOf<I, E>) -> PResult<I, Vec<O>, E> {
        #[allow(deprecated)]
        self.parse_inner(d, s)
    }
}

/// See [`Parser::debug`].
pub struct Debug<A>(
    pub(crate) A,
    pub(crate) Rc<dyn fmt::Display>,
    pub(crate) std::panic::Location<'static>,
);

impl<A: Clone> Clone for Debug<A> {
    fn clone(&self) -> Self {
        Self(self.0.clone(), self.1.clone(), self.2)
    }
}

impl<I: Clone, O, A: Parser<I, O, Error = E>, E: Error<I>> Parser<I, O> for Debug<A> {
    type Error = E;

    #[inline]
    fn parse_inner<D: Debugger>(
        &self,
        debugger: &mut D,
        stream: &mut StreamOf<I, E>,
    ) -> PResult<I, O, E> {
        debugger.scope(
            || ParserInfo::new("Name", self.1.clone(), self.2),
            |debugger| {
                #[allow(deprecated)]
                let (errors, res) = debugger.invoke(&self.0, stream);

                (errors, res)
            },
        )
    }

    #[inline]
    fn parse_inner_verbose(&self, d: &mut Verbose, s: &mut StreamOf<I, E>) -> PResult<I, O, E> {
        #[allow(deprecated)]
        self.parse_inner(d, s)
    }
    #[inline]
    fn parse_inner_silent(&self, d: &mut Silent, s: &mut StreamOf<I, E>) -> PResult<I, O, E> {
        #[allow(deprecated)]
        self.parse_inner(d, s)
    }
}

/// See [`Parser::map`].
pub struct Map<A, F, O>(pub(crate) A, pub(crate) F, pub(crate) PhantomData<O>);

impl<A: Copy, F: Copy, O> Copy for Map<A, F, O> {}
impl<A: Clone, F: Clone, O> Clone for Map<A, F, O> {
    fn clone(&self) -> Self {
        Self(self.0.clone(), self.1.clone(), PhantomData)
    }
}

impl<I: Clone, O, A: Parser<I, O, Error = E>, U, F: Fn(O) -> U, E: Error<I>> Parser<I, U>
    for Map<A, F, O>
{
    type Error = E;

    #[inline]
    fn parse_inner<D: Debugger>(
        &self,
        debugger: &mut D,
        stream: &mut StreamOf<I, E>,
    ) -> PResult<I, U, E> {
        #[allow(deprecated)]
        let (errors, res) = debugger.invoke(&self.0, stream);

        (errors, res.map(|(out, alt)| ((&self.1)(out), alt)))
    }

    #[inline]
    fn parse_inner_verbose(&self, d: &mut Verbose, s: &mut StreamOf<I, E>) -> PResult<I, U, E> {
        #[allow(deprecated)]
        self.parse_inner(d, s)
    }
    #[inline]
    fn parse_inner_silent(&self, d: &mut Silent, s: &mut StreamOf<I, E>) -> PResult<I, U, E> {
        #[allow(deprecated)]
        self.parse_inner(d, s)
    }
}

/// See [`Parser::map_with_span`].
pub struct MapWithSpan<A, F, O>(pub(crate) A, pub(crate) F, pub(crate) PhantomData<O>);

impl<A: Copy, F: Copy, O> Copy for MapWithSpan<A, F, O> {}
impl<A: Clone, F: Clone, O> Clone for MapWithSpan<A, F, O> {
    fn clone(&self) -> Self {
        Self(self.0.clone(), self.1.clone(), PhantomData)
    }
}

impl<I: Clone, O, A: Parser<I, O, Error = E>, U, F: Fn(O, E::Span) -> U, E: Error<I>> Parser<I, U>
    for MapWithSpan<A, F, O>
{
    type Error = E;

    #[inline]
    fn parse_inner<D: Debugger>(
        &self,
        debugger: &mut D,
        stream: &mut StreamOf<I, E>,
    ) -> PResult<I, U, E> {
        let start = stream.save();
        #[allow(deprecated)]
        let (errors, res) = debugger.invoke(&self.0, stream);

        (
            errors,
            res.map(|(out, alt)| ((self.1)(out, stream.span_since(start)), alt)),
        )
    }

    #[inline]
    fn parse_inner_verbose(&self, d: &mut Verbose, s: &mut StreamOf<I, E>) -> PResult<I, U, E> {
        #[allow(deprecated)]
        self.parse_inner(d, s)
    }
    #[inline]
    fn parse_inner_silent(&self, d: &mut Silent, s: &mut StreamOf<I, E>) -> PResult<I, U, E> {
        #[allow(deprecated)]
        self.parse_inner(d, s)
    }
}

/// See [`Parser::validate`].
#[derive(Copy, Clone)]
pub struct Validate<A, F>(pub(crate) A, pub(crate) F);

impl<
        I: Clone,
        O,
        A: Parser<I, O, Error = E>,
        F: Fn(O, E::Span, &mut dyn FnMut(E)) -> O,
        E: Error<I>,
    > Parser<I, O> for Validate<A, F>
{
    type Error = E;

    #[inline]
    fn parse_inner<D: Debugger>(
        &self,
        debugger: &mut D,
        stream: &mut StreamOf<I, E>,
    ) -> PResult<I, O, E> {
        let start = stream.save();
        #[allow(deprecated)]
        let (mut errors, res) = debugger.invoke(&self.0, stream);

        let pos = stream.save();
        let span = stream.span_since(start);

        let res = res.map(|(out, alt)| {
            (
                (&self.1)(out, span, &mut |e| errors.push(Located::at(pos, e))),
                alt,
            )
        });

        (errors, res)
    }

    #[inline]
    fn parse_inner_verbose(&self, d: &mut Verbose, s: &mut StreamOf<I, E>) -> PResult<I, O, E> {
        #[allow(deprecated)]
        self.parse_inner(d, s)
    }
    #[inline]
    fn parse_inner_silent(&self, d: &mut Silent, s: &mut StreamOf<I, E>) -> PResult<I, O, E> {
        #[allow(deprecated)]
        self.parse_inner(d, s)
    }
}

/// See [`Parser::foldl`].
pub struct Foldl<A, F, O, U>(pub(crate) A, pub(crate) F, pub(crate) PhantomData<(O, U)>);

impl<A: Copy, F: Copy, O, U> Copy for Foldl<A, F, O, U> {}
impl<A: Clone, F: Clone, O, U> Clone for Foldl<A, F, O, U> {
    fn clone(&self) -> Self {
        Self(self.0.clone(), self.1.clone(), PhantomData)
    }
}

impl<
        I: Clone,
        O,
        A: Parser<I, (O, U), Error = E>,
        U: IntoIterator,
        F: Fn(O, U::Item) -> O,
        E: Error<I>,
    > Parser<I, O> for Foldl<A, F, O, U>
{
    type Error = E;

    #[inline]
    fn parse_inner<D: Debugger>(
        &self,
        debugger: &mut D,
        stream: &mut StreamOf<I, E>,
    ) -> PResult<I, O, E> {
        #[allow(deprecated)]
        debugger.invoke(
            &(&self.0).map(|(head, tail)| tail.into_iter().fold(head, &self.1)),
            stream,
        )
    }

    #[inline]
    fn parse_inner_verbose(&self, d: &mut Verbose, s: &mut StreamOf<I, E>) -> PResult<I, O, E> {
        #[allow(deprecated)]
        self.parse_inner(d, s)
    }
    #[inline]
    fn parse_inner_silent(&self, d: &mut Silent, s: &mut StreamOf<I, E>) -> PResult<I, O, E> {
        #[allow(deprecated)]
        self.parse_inner(d, s)
    }
}

/// See [`Parser::foldr`].
pub struct Foldr<A, F, O, U>(pub(crate) A, pub(crate) F, pub(crate) PhantomData<(O, U)>);

impl<A: Copy, F: Copy, O, U> Copy for Foldr<A, F, O, U> {}
impl<A: Clone, F: Clone, O, U> Clone for Foldr<A, F, O, U> {
    fn clone(&self) -> Self {
        Self(self.0.clone(), self.1.clone(), PhantomData)
    }
}

impl<
        I: Clone,
        O: IntoIterator,
        A: Parser<I, (O, U), Error = E>,
        U,
        F: Fn(O::Item, U) -> U,
        E: Error<I>,
    > Parser<I, U> for Foldr<A, F, O, U>
where
    O::IntoIter: DoubleEndedIterator,
{
    type Error = E;

    #[inline]
    fn parse_inner<D: Debugger>(
        &self,
        debugger: &mut D,
        stream: &mut StreamOf<I, E>,
    ) -> PResult<I, U, E> {
        #[allow(deprecated)]
        debugger.invoke(
            &(&self.0).map(|(init, end)| init.into_iter().rev().fold(end, |b, a| (&self.1)(a, b))),
            stream,
        )
    }

    #[inline]
    fn parse_inner_verbose(&self, d: &mut Verbose, s: &mut StreamOf<I, E>) -> PResult<I, U, E> {
        #[allow(deprecated)]
        self.parse_inner(d, s)
    }
    #[inline]
    fn parse_inner_silent(&self, d: &mut Silent, s: &mut StreamOf<I, E>) -> PResult<I, U, E> {
        #[allow(deprecated)]
        self.parse_inner(d, s)
    }
}

/// See [`Parser::map_err`].
#[derive(Copy, Clone)]
pub struct MapErr<A, F>(pub(crate) A, pub(crate) F);

impl<I: Clone, O, A: Parser<I, O, Error = E>, F: Fn(E) -> E, E: Error<I>> Parser<I, O>
    for MapErr<A, F>
{
    type Error = E;

    #[inline]
    fn parse_inner<D: Debugger>(
        &self,
        debugger: &mut D,
        stream: &mut StreamOf<I, E>,
    ) -> PResult<I, O, E> {
        #[allow(deprecated)]
        let (errors, res) = debugger.invoke(&self.0, stream);
        let mapper = |e: Located<I, E>| e.map(&self.1);
        (
            errors.into_iter().map(mapper).collect(),
            res.map(|(out, alt)| (out, alt.map(mapper))).map_err(mapper),
        )
    }

    #[inline]
    fn parse_inner_verbose(&self, d: &mut Verbose, s: &mut StreamOf<I, E>) -> PResult<I, O, E> {
        #[allow(deprecated)]
        self.parse_inner(d, s)
    }
    #[inline]
    fn parse_inner_silent(&self, d: &mut Silent, s: &mut StreamOf<I, E>) -> PResult<I, O, E> {
        #[allow(deprecated)]
        self.parse_inner(d, s)
    }
}

/// See [`Parser::try_map`].
pub struct TryMap<A, F, O>(pub(crate) A, pub(crate) F, pub(crate) PhantomData<O>);

impl<A: Copy, F: Copy, O> Copy for TryMap<A, F, O> {}
impl<A: Clone, F: Clone, O> Clone for TryMap<A, F, O> {
    fn clone(&self) -> Self {
        Self(self.0.clone(), self.1.clone(), PhantomData)
    }
}

impl<
        I: Clone,
        O,
        A: Parser<I, O, Error = E>,
        U,
        F: Fn(O, E::Span) -> Result<U, E>,
        E: Error<I>,
    > Parser<I, U> for TryMap<A, F, O>
{
    type Error = E;

    #[inline]
    fn parse_inner<D: Debugger>(
        &self,
        debugger: &mut D,
        stream: &mut StreamOf<I, E>,
    ) -> PResult<I, U, E> {
        let start = stream.save();
        #[allow(deprecated)]
        let (errors, res) = debugger.invoke(&self.0, stream);

        let res = match res.map(|(out, alt)| ((&self.1)(out, stream.span_since(start)), alt)) {
            Ok((Ok(out), alt)) => Ok((out, alt)),
            Ok((Err(a_err), _)) => Err(Located::at(stream.save(), a_err)),
            Err(err) => Err(err),
        };

        (errors, res)
    }

    #[inline]
    fn parse_inner_verbose(&self, d: &mut Verbose, s: &mut StreamOf<I, E>) -> PResult<I, U, E> {
        #[allow(deprecated)]
        self.parse_inner(d, s)
    }
    #[inline]
    fn parse_inner_silent(&self, d: &mut Silent, s: &mut StreamOf<I, E>) -> PResult<I, U, E> {
        #[allow(deprecated)]
        self.parse_inner(d, s)
    }
}

/// See [`Parser::labelled`].
#[derive(Copy, Clone)]
pub struct Label<A, L>(pub(crate) A, pub(crate) L);

impl<I: Clone, O, A: Parser<I, O, Error = E>, L: Into<E::Label> + Clone, E: Error<I>> Parser<I, O>
    for Label<A, L>
{
    type Error = E;

    #[inline]
    fn parse_inner<D: Debugger>(
        &self,
        debugger: &mut D,
        stream: &mut StreamOf<I, E>,
    ) -> PResult<I, O, E> {
        let pre_state = stream.save();
        #[allow(deprecated)]
        let (errors, res) = debugger.invoke(&self.0, stream);
        let res = res.map_err(|e| {
            if e.at > pre_state || true
            /* TODO: Not this? */
            {
                // Only add the label if we committed to this pattern somewhat
                e.map(|e| e.with_label(self.1.clone().into()))
            } else {
                e
            }
        });
        (
            errors
                .into_iter()
                .map(|e| e.map(|e| e.with_label(self.1.clone().into())))
                .collect(),
            res,
        )
    }

    #[inline]
    fn parse_inner_verbose(&self, d: &mut Verbose, s: &mut StreamOf<I, E>) -> PResult<I, O, E> {
        #[allow(deprecated)]
        self.parse_inner(d, s)
    }
    #[inline]
    fn parse_inner_silent(&self, d: &mut Silent, s: &mut StreamOf<I, E>) -> PResult<I, O, E> {
        #[allow(deprecated)]
        self.parse_inner(d, s)
    }
}

/// See [`Parser::to`].
pub struct To<A, O, U>(pub(crate) A, pub(crate) U, pub(crate) PhantomData<O>);

impl<A: Copy, U: Copy, O> Copy for To<A, O, U> {}
impl<A: Clone, U: Clone, O> Clone for To<A, O, U> {
    fn clone(&self) -> Self {
        Self(self.0.clone(), self.1.clone(), PhantomData)
    }
}

impl<I: Clone, O, A: Parser<I, O, Error = E>, U: Clone, E: Error<I>> Parser<I, U> for To<A, O, U> {
    type Error = E;

    #[inline]
    fn parse_inner<D: Debugger>(
        &self,
        debugger: &mut D,
        stream: &mut StreamOf<I, E>,
    ) -> PResult<I, U, E> {
        #[allow(deprecated)]
        debugger.invoke(&(&self.0).map(|_| self.1.clone()), stream)
    }

    #[inline]
    fn parse_inner_verbose(&self, d: &mut Verbose, s: &mut StreamOf<I, E>) -> PResult<I, U, E> {
        #[allow(deprecated)]
        self.parse_inner(d, s)
    }
    #[inline]
    fn parse_inner_silent(&self, d: &mut Silent, s: &mut StreamOf<I, E>) -> PResult<I, U, E> {
        #[allow(deprecated)]
        self.parse_inner(d, s)
    }
}

/// See [`Parser::lookahead`].
pub struct Lookahead<A>(pub(crate) A);

impl<I: Clone, O, E: Error<I>, A> Parser<I, O> for Lookahead<A>
where
    A: Parser<I, O, Error = E>,
{
    type Error = E;

    fn parse_inner<D: Debugger>(
        &self,
        debugger: &mut D,
        stream: &mut StreamOf<I, Self::Error>,
    ) -> PResult<I, O, Self::Error>
    where
        Self: Sized,
    {
        let lookahead_from = stream.save();
        match {
            #[allow(deprecated)]
            debugger.invoke(&self.0, stream)
        } {
            (errors, Ok((out, alt))) => {
                stream.revert(lookahead_from);
                (errors, Ok((out, alt)))
            }
            (errors, Err(err)) => (errors, Err(err)),
        }
    }

    fn parse_inner_verbose(
        &self,
        d: &mut Verbose,
        s: &mut StreamOf<I, Self::Error>,
    ) -> PResult<I, O, Self::Error> {
        #[allow(deprecated)]
        self.parse_inner(d, s)
    }

    fn parse_inner_silent(
        &self,
        d: &mut Silent,
        s: &mut StreamOf<I, Self::Error>,
    ) -> PResult<I, O, Self::Error> {
        #[allow(deprecated)]
        self.parse_inner(d, s)
    }
}

#[cfg(test)]
mod tests {
    use error::Simple;

    use super::*;

    #[test]
    fn separated_by_at_least() {
        let parser = just::<_, Simple<char>>('-')
            .separated_by(just(','))
            .at_least(3);

        assert_eq!(parser.parse("-,-,-"), Ok(vec!['-', '-', '-']));
    }

    #[test]
    fn separated_by_at_least_without_leading() {
        let parser = just::<_, Simple<char>>('-')
            .separated_by(just(','))
            .at_least(3);

        assert!(parser.parse(",-,-,-").is_err());
    }

    #[test]
    fn separated_by_at_least_without_trailing() {
        let parser = just::<_, Simple<char>>('-')
            .separated_by(just(','))
            .at_least(3)
            .then(end());

        assert!(parser.parse("-,-,-,").is_err());
    }

    #[test]
    fn separated_by_at_least_with_leading() {
        let parser = just::<_, Simple<char>>('-')
            .separated_by(just(','))
            .allow_leading()
            .at_least(3);

        assert_eq!(parser.parse(",-,-,-"), Ok(vec!['-', '-', '-']));
        assert!(parser.parse(",-,-").is_err());
    }

    #[test]
    fn separated_by_at_least_with_trailing() {
        let parser = just::<_, Simple<char>>('-')
            .separated_by(just(','))
            .allow_trailing()
            .at_least(3);

        assert_eq!(parser.parse("-,-,-,"), Ok(vec!['-', '-', '-']));
        assert!(parser.parse("-,-,").is_err());
    }

    #[test]
    fn separated_by_leaves_last_separator() {
        let parser = just::<_, Simple<char>>('-')
            .separated_by(just(','))
            .chain(just(','));
        assert_eq!(parser.parse("-,-,-,"), Ok(vec!['-', '-', '-', ',']))
    }
}
