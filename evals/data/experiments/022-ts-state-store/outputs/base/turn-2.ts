export interface Product {
  id: string;
  name: string;
  price: number;
}

export interface CartItem extends Product {
  quantity: number;
}

export interface CartState {
  items: CartItem[];
  couponCode: string | null;
  discountRate: number;
  taxRate: number;
  bulkDiscountApplied: boolean;
}

export type CartAction =
  | { type: 'ADD_ITEM'; payload: Product }
  | { type: 'REMOVE_ITEM'; payload: string }
  | { type: 'UPDATE_QUANTITY'; payload: { id: string; quantity: number } }
  | { type: 'APPLY_COUPON'; payload: { code: string; discount: number } }
  | { type: 'APPLY_BULK_DISCOUNT' }
  | { type: 'CLEAR_CART' };

export const initialState: CartState = {
  items: [],
  couponCode: null,
  discountRate: 0,
  taxRate: 0.08,
  bulkDiscountApplied: false,
};

export const addItem = (product: Product): CartAction => ({ type: 'ADD_ITEM', payload: product });
export const removeItem = (id: string): CartAction => ({ type: 'REMOVE_ITEM', payload: id });
export const updateQuantity = (id: string, quantity: number): CartAction => ({ type: 'UPDATE_QUANTITY', payload: { id, quantity } });
export const applyCoupon = (code: string, discount: number): CartAction => ({ type: 'APPLY_COUPON', payload: { code, discount } });
export const applyBulkDiscount = (): CartAction => ({ type: 'APPLY_BULK_DISCOUNT' });
export const clearCart = (): CartAction => ({ type: 'CLEAR_CART' });

export const cartReducer = (state: CartState, action: CartAction): CartState => {
  switch (action.type) {
    case 'ADD_ITEM': {
      const existingItem = state.items.find((item) => item.id === action.payload.id);
      return {
        ...state,
        items: existingItem 
          ? state.items.map((item) => item.id === action.payload.id ? { ...item, quantity: item.quantity + 1 } : item)
          : [...state.items, { ...action.payload, quantity: 1 }]
      };
    }
    case 'REMOVE_ITEM':
      return { ...state, items: state.items.filter((item) => item.id !== action.payload) };
    case 'UPDATE_QUANTITY':
      return {
        ...state,
        items: state.items.map((item) => item.id === action.payload.id ? { ...item, quantity: action.payload.quantity } : item)
      };
    case 'APPLY_COUPON':
      return { ...state, couponCode: action.payload.code, discountRate: action.payload.discount };
    case 'APPLY_BULK_DISCOUNT': {
      const totalItems = state.items.reduce((acc, item) => acc + item.quantity, 0);
      return totalItems >= 5 
        ? { ...state, discountRate: 0.15, bulkDiscountApplied: true }
        : state;
    }
    case 'CLEAR_CART':
      return { ...initialState };
    default:
      return state;
  }
};

export const getCartItems = (state: CartState) => state.items;
export const getItemCount = (state: CartState) => state.items.reduce((acc, item) => acc + item.quantity, 0);
export const getCartTotal = (state: CartState) => {
  const subtotal = state.items.reduce((acc, item) => acc + item.price * item.quantity, 0);
  return subtotal * (1 - state.discountRate);
};
export const getCouponDiscount = (state: CartState) => 
  state.items.reduce((acc, item) => acc + item.price * item.quantity, 0) * state.discountRate;

export const getTaxAmount = (state: CartState, customTaxRate?: number) => {
  const effectiveTaxRate = customTaxRate !== undefined ? customTaxRate : state.taxRate;
  return getCartTotal(state) * effectiveTaxRate;
};

export const loadState = (): CartState => {
  try {
    const serializedState = localStorage.getItem('cart_state');
    return serializedState ? JSON.parse(serializedState) : initialState;
  } catch { return initialState; }
};

export const saveState = (state: CartState) => {
  try {
    localStorage.setItem('cart_state', JSON.stringify(state));
  } catch (err) { console.error("Could not save state", err); }
};