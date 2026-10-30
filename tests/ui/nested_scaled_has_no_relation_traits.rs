use numo::{
  ConvertTo, Decimal, DivRound, ExactMul, Scaled, WidenTo,
  typenum::{U2, U4, U8, U10},
};

// A `Scaled` over a decimal relates to another such `Scaled`, but a `Scaled` over a `Scaled` is not
// a decimal and has none of the traits between decimal types.
fn widens<A: WidenTo<B>, B>() {}

fn converts<A: ConvertTo<B>, B>() {}

fn divides<A: DivRound<B, C>, B, C>() {}

fn multiplies<A: ExactMul<B>, B>() {}

type Narrow = Decimal<U2, U4>;
type Wide = Decimal<U10, U8>;

fn main() {
  widens::<Scaled<Narrow>, Scaled<Wide>>();
  converts::<Scaled<Wide>, Scaled<Narrow>>();
  divides::<Scaled<Wide>, Scaled<Narrow>, Scaled<Wide>>();
  multiplies::<Scaled<Narrow>, Scaled<Wide>>();

  widens::<Scaled<Scaled<Narrow>>, Scaled<Scaled<Wide>>>();
  converts::<Scaled<Scaled<Wide>>, Scaled<Scaled<Narrow>>>();
  divides::<Scaled<Scaled<Wide>>, Scaled<Scaled<Narrow>>, Scaled<Scaled<Wide>>>();
  multiplies::<Scaled<Scaled<Narrow>>, Scaled<Scaled<Wide>>>();
}
