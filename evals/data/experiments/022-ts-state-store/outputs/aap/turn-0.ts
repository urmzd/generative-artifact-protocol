<aap:target id="cart-store">
export interface Product {
  id: string;
  name: string;
  price: number;
}

export interface CartItem extends Product {
  quantity: number;
}

export interface CartState {
  <aap:target id="items">items: CartItem[];</aap:target>
  <aap:target id="coupon-code">couponCode: string | null;</aap:target>
  <aap:target id="tax-rate">taxRate: number;</aap:target>
}

export type CartAction =
  | { type: 'ADD_ITEM'; payload: Product }
  | { type: 'REMOVE_ITEM'; payload: string }
  | { type: 'UPDATE_QUANTITY'; payload: { id: string; quantity: number } }
  | { type: 'APPLY_COUPON'; payload: string | null }
  | { type: 'CLEAR_CART' };

export const initialState: CartState = {
  items: [],
  couponCode: null,
  taxRate: 0.08,
};

export const cartReducer = (state: CartState, action: CartAction): CartState => {
  switch (action.type) {
    case 'ADD_ITEM':
      const existing = state.items.find(i => i.id === action.payload.id);
      return {
        ...state,
        items: existing
          ? state.items.map(i => i.id === action.payload.id ? { ...i, quantity: i.quantity + 1 } : i)
          : [...state.items, { ...action.payload, quantity: 1 }]
      };
    case 'REMOVE_ITEM':
      return { ...state, items: state.items.filter(i => i.id !== action.payload) };
    case 'UPDATE_QUANTITY':
      return {
        ...state,
        items: state.items.map(i => i.id === action.payload.id ? { ...i, quantity: action.payload.quantity } : i)
      };
    case 'APPLY_COUPON':
      return { ...state, couponCode: action.payload };
    case 'CLEAR_CART':
      return { ...initialState };
    default:
      return state;
  }
};

export const selectors = {
  getCartItems: (state: CartState) => state.items,
  getItemCount: (state: CartState) => state.items.reduce((acc, item) => acc + item.quantity, 0),
  getCartTotal: (state: CartState) => state.items.reduce((acc, item) => acc + (item.price * item.quantity), 0),
  getCouponDiscount: (state: CartState) => state.couponCode === 'SAVE10' ? 0.10 : 0,
  getTaxAmount: (state: CartState) => {
    const subtotal = state.items.reduce((acc, item) => acc + (item.price * item.quantity), 0);
    return subtotal * state.taxRate;
  }
};

export const persistence = {
  save: (state: CartState) => localStorage.setItem('cart', JSON.stringify(state)),
  load: (): CartState | null => {
    const data = localStorage.getItem('cart');
    return data ? JSON.parse(data) : null;
  }
};
</aap:target>