{
  "protocol": "aap/0.1",
  "id": "cart-store",
  "version": 2,
  "name": "edit",
  "content": [
    {
      "op": "insert_after",
      "target": { "type": "id", "value": "coupon-code" },
      "content": "  <aap:target id=\"bulk-discount-applied\">bulkDiscountApplied: boolean;</aap:target>"
    },
    {
      "op": "insert_after",
      "target": { "type": "id", "value": "items" },
      "content": "  <aap:target id=\"bulk-discount-applied\">bulkDiscountApplied: boolean;</aap:target>"
    },
    {
      "op": "replace",
      "target": { "type": "id", "value": "items" },
      "content": "items: CartItem[];\n  bulkDiscountApplied: boolean;"
    },
    {
      "op": "insert_before",
      "target": { "type": "id", "value": "coupon-code" },
      "content": "  bulkDiscountApplied: boolean;"
    },
    {
      "op": "replace",
      "target": { "type": "id", "value": "items" },
      "content": "items: CartItem[];"
    },
    {
      "op": "replace",
      "target": { "type": "id", "value": "coupon-code" },
      "content": "couponCode: string | null;\n  bulkDiscountApplied: boolean;"
    },
    {
      "op": "insert_before",
      "target": { "type": "id", "value": "CLEAR_CART" },
      "content": "  | { type: 'APPLY_BULK_DISCOUNT' }"
    },
    {
      "op": "replace",
      "target": { "type": "id", "value": "initialState" },
      "content": "export const initialState: CartState = {\n  items: [],\n  couponCode: null,\n  bulkDiscountApplied: false,\n  taxRate: 0.08,\n};"
    },
    {
      "op": "insert_before",
      "target": { "type": "id", "value": "case 'APPLY_COUPON'" },
      "content": "    case 'APPLY_BULK_DISCOUNT':\n      const totalItems = state.items.reduce((acc, i) => acc + i.quantity, 0);\n      return { ...state, bulkDiscountApplied: totalItems >= 5 };"
    }
  ]
}