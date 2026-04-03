interface CartState {
  <aap:target id="items">items: CartItem[];</aap:target>
  <aap:target id="coupon-code">couponCode: string | null;
  bulkDiscountApplied: boolean;</aap:target>
  taxRate: number;
}

export const initialState: CartState = {
  items: [],
  couponCode: null,
  bulkDiscountApplied: false,
  taxRate: 0.08,
};

type CartAction =
  | { type: 'ADD_ITEM'; payload: CartItem }
  | { type: 'REMOVE_ITEM'; payload: string }
  | { type: 'APPLY_COUPON'; payload: string }
  | { type: 'APPLY_BULK_DISCOUNT' }
  | <aap:target id="CLEAR_CART">{ type: 'CLEAR_CART' }</aap:target>;

export function cartReducer(state: CartState, action: CartAction): CartState {
  switch (action.type) {
    case 'APPLY_BULK_DISCOUNT':
      const totalItems = state.items.reduce((acc, i) => acc + i.quantity, 0);
      return { ...state, bulkDiscountApplied: totalItems >= 5 };
    case 'APPLY_COUPON':
      return { ...state, couponCode: action.payload };
    case 'CLEAR_CART':
      return initialState;
    default:
      return state;
  }
}

export const getSubtotal = (items: CartItem[]) => items.reduce((sum, item) => sum + item.price * item.quantity, 0);

export const getTaxAmount = (state: CartState, taxRate: number) => getSubtotal(state.items) * taxRate;