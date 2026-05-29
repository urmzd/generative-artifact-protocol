<gap:target id="product-management-module">
package com.example.productmanagement;

<gap:target id="controller-section">
    /**
     * Export all products as CSV.
     *
     * @return CSV file download
     */
    @GetMapping("/export")
    public ResponseEntity<Resource> exportProducts() {
        Resource csvFile = productService.exportProductsAsCsv();
        return ResponseEntity.ok()
                .header(HttpHeaders.CONTENT_DISPOSITION, "attachment; filename=products.csv")
                .contentType(MediaType.parseMediaType("text/csv"))
                .body(csvFile);
    }

/**
 * REST controller for product management operations.
 */
@RestController
@RequestMapping("<gap:target id="product-base-path">/api/products</gap:target>")
@Validated
public class ProductController {

    private final ProductService productService;

    public ProductController(ProductService productService) {
        this.productService = productService;
    }

    /**
     * Create a new product.
     *
     * @param request the product request payload
     * @return the created product
     */
    @PostMapping
    public ResponseEntity<ProductResponse> createProduct(
            @Valid @RequestBody ProductRequest request) {
        ProductResponse response = productService.createProduct(request);
        return ResponseEntity.status(HttpStatus.CREATED).body(response);
    }

    /**
     * Get a product by id.
     *
     * @param id the product id
     * @return the product response
     */
    @GetMapping("/{id}")
    public ResponseEntity<ProductResponse> getProductById(
            @PathVariable Long id) {
        return ResponseEntity.ok(productService.getProductById(id));
    }

    /**
     * Get all products with pagination and sorting.
     *
     * @param page page number
     * @param size page size
     * @param sortBy sort field
     * @param sortDir sort direction
     * @return paged response of products
     */
    @GetMapping
    public ResponseEntity<PagedResponse<ProductResponse>> getAllProducts(
            @RequestParam(defaultValue = "<gap:target id="default-page-number">0</gap:target>") int page,
            @RequestParam(defaultValue = "<gap:target id="default-page-size">10</gap:target>") int size,
            @RequestParam(defaultValue = "<gap:target id="default-sort-field">name</gap:target>") String sortBy,
            @RequestParam(defaultValue = "<gap:target id="default-sort-direction">asc</gap:target>") String sortDir) {
        return ResponseEntity.ok(productService.getAllProducts(page, size, sortBy, sortDir));
    }

    /**
     * Search products by criteria.
     *
     * @param criteria the search criteria
     * @return paged response of matching products
     */
    @PostMapping("/search")
    public ResponseEntity<PagedResponse<ProductResponse>> searchProducts(
            @Valid @RequestBody ProductSearchCriteria criteria) {
        return ResponseEntity.ok(productService.searchProducts(criteria));
    }

    /**
     * Update an existing product.
     *
     * @param id the product id
     * @param request the product request payload
     * @return the updated product
     */
    @PutMapping("/{id}")
    public ResponseEntity<ProductResponse> updateProduct(
            @PathVariable Long id,
            @Valid @RequestBody ProductRequest request) {
        return ResponseEntity.ok(productService.updateProduct(id, request));
    }

    /**
     * Delete a product by id.
     *
     * @param id the product id
     * @return no content
     */
    @DeleteMapping("/{id}")
    public ResponseEntity<Void> deleteProduct(@PathVariable Long id) {
        productService.deleteProduct(id);
        return ResponseEntity.noContent().build();
    }
}
</gap:target>

<gap:target id="service-section">
/**
 * Business service for product operations.
 */
@Service
@Validated
public class ProductService {

    private final ProductRepository productRepository;

    public ProductService(ProductRepository productRepository) {
        this.productRepository = productRepository;
    }

    /**
     * Create a new product.
     *
     * @param request the product request
     * @return product response
     */
    @CacheEvict(value = "<gap:target id="product-cache-name">products</gap:target>", allEntries = true)
    public ProductResponse createProduct(@Valid ProductRequest request) {
        validateSkuUniqueness(request.getSku(), null);
        Product product = mapToEntity(request);
        Product saved = productRepository.save(product);
        return mapToResponse(saved);
    }

    /**
     * Get a product by id.
     *
     * @param id product id
     * @return product response
     */
    @Cacheable(value = "products", key = "#id")
    public ProductResponse getProductById(Long id) {
        Product product = productRepository.findById(id)
                .orElseThrow(() -> new ResourceNotFoundException("Product not found with id: " + id));
        return mapToResponse(product);
    }

    /**
     * Get all products with pagination and sorting.
     *
     * @param page page number
     * @param size page size
     * @param sortBy sort field
     * @param sortDir sort direction
     * @return paged response
     */
    @Cacheable(value = "products", key = "'all:' + #page + ':' + #size + ':' + #sortBy + ':' + #sortDir")
    public PagedResponse<ProductResponse> getAllProducts(int page, int size, String sortBy, String sortDir) {
        Sort sort = sortDir.equalsIgnoreCase("desc")
                ? Sort.by(sortBy).descending()
                : Sort.by(sortBy).ascending();

        Pageable pageable = PageRequest.of(page, size, sort);
        Page<Product> productPage = productRepository.findAll(pageable);
        return mapToPagedResponse(productPage);
    }

    /**
     * Search products by criteria.
     *
     * @param criteria search criteria
     * @return paged response
     */
    @Cacheable(value = "products", key = "'search:' + #criteria.keyword + ':' + #criteria.category + ':' + #criteria.minPrice + ':' + #criteria.maxPrice + ':' + #criteria.page + ':' + #criteria.size + ':' + #criteria.sortBy + ':' + #criteria.sortDir")
    public PagedResponse<ProductResponse> searchProducts(@Valid ProductSearchCriteria criteria) {
        Sort sort = criteria.getSortDir().equalsIgnoreCase("desc")
                ? Sort.by(criteria.getSortBy()).descending()
                : Sort.by(criteria.getSortBy()).ascending();

        Pageable pageable = PageRequest.of(criteria.getPage(), criteria.getSize(), sort);
        Page<Product> page = productRepository.searchProducts(
                criteria.getKeyword(),
                criteria.getCategory(),
                criteria.getMinPrice(),
                criteria.getMaxPrice(),
                pageable
        );
        return mapToPagedResponse(page);
    }

    /**
     * Update a product.
     *
     * @param id product id
     * @param request request payload
     * @return updated product response
     */
    @CacheEvict(value = "products", allEntries = true)
    public ProductResponse updateProduct(Long id, @Valid ProductRequest request) {
        Product existing = productRepository.findById(id)
                .orElseThrow(() -> new ResourceNotFoundException("Product not found with id: " + id));

        validateSkuUniqueness(request.getSku(), id);
        existing.setName(request.getName());
        existing.setSku(request.getSku());
        existing.setDescription(request.getDescription());
        existing.setCategory(request.getCategory());
        existing.setPrice(request.getPrice());
        existing.setStockQuantity(request.getStockQuantity());
        existing.setActive(request.getActive());

        return mapToResponse(productRepository.save(existing));
    }

    /**
     * Delete a product.
     *
     * @param id product id
     */
    @CacheEvict(value = "products", allEntries = true)
    public void deleteProduct(Long id) {
        if (!productRepository.existsById(id)) {
            throw new ResourceNotFoundException("Product not found with id: " + id);
        }
        productRepository.deleteById(id);
    }

    private void validateSkuUniqueness(String sku, Long excludeId) {
        boolean exists = excludeId == null
                ? productRepository.existsBySku(sku)
                : productRepository.existsBySkuAndIdNot(sku, excludeId);
        if (exists) {
            throw new BadRequestException("SKU already exists: " + sku);
        }
    }

    private Product mapToEntity(ProductRequest request) {
        Product product = new Product();
        product.setName(request.getName());
        product.setSku(request.getSku());
        product.setDescription(request.getDescription());
        product.setCategory(request.getCategory());
        product.setPrice(request.getPrice());
        product.setStockQuantity(request.getStockQuantity());
        product.setActive(request.getActive());
        return product;
    }

    private ProductResponse mapToResponse(Product product) {
        return new ProductResponse(
                product.getId(),
                product.getName(),
                product.getSku(),
                product.getDescription(),
                product.getCategory(),
                product.getPrice(),
                product.getStockQuantity(),
                product.getActive(),
                product.getCreatedAt(),
                product.getUpdatedAt()
        );
    }

    private PagedResponse<ProductResponse> mapToPagedResponse(Page<Product> page) {
        List<ProductResponse> content = page.getContent()
                .stream()
                .map(this::mapToResponse)
                .toList();

        return new PagedResponse<>(
                content,
                page.getNumber(),
                page.getSize(),
                page.getTotalElements(),
                page.getTotalPages(),
                page.isLast()
        );
    }
}
</gap:target>

<gap:target id="repository-section">/**
 * JPA repository for product persistence operations.
 */
@Repository
public interface ProductRepository extends JpaRepository<Product, Long>, JpaSpecificationExecutor<Product> {

    /**
     * Find product by SKU.
     *
     * @param sku product SKU
     * @return true if exists
     */
    boolean existsBySku(String sku);

    /**
     * Check if SKU exists excluding a specific id.
     *
     * @param sku product SKU
     * @param id id to exclude
     * @return true if exists
     */
    boolean existsBySkuAndIdNot(String sku, Long id);

    /**
     * Find products by exact category.
     *
     * @param category product category
     * @return matching products
     */
    List<Product> findByCategory(String category);

    /**
     * Find products by name containing keyword.
     *
     * @param name product name keyword
     * @return matching products
     */
    List<Product> findByNameContainingIgnoreCase(String name);

    /**
     * Find products by price range and category.
     *
     * @param minPrice minimum price
     * @param maxPrice maximum price
     * @param category product category
     * @return matching products
     */
    @Query("""
           SELECT p
           FROM Product p
           WHERE p.price BETWEEN :minPrice AND :maxPrice
             AND (:category IS NULL OR LOWER(p.category) = LOWER(:category))
           """)
    List<Product> findByPriceRangeAndCategory(@Param("minPrice") BigDecimal minPrice,
                                              @Param("maxPrice") BigDecimal maxPrice,
                                              @Param("category") String category);

    /**
     * Custom search query for products.
     *
     * @param keyword search keyword
     * @param category category filter
     * @param minPrice minimum price
     * @param maxPrice maximum price
     * @param pageable pageable request
     * @return page of products
     */
    @Query("""
           SELECT p
           FROM Product p
           WHERE (:keyword IS NULL OR LOWER(p.name) LIKE LOWER(CONCAT('%', :keyword, '%'))
               OR LOWER(p.description) LIKE LOWER(CONCAT('%', :keyword, '%')))
             AND (:category IS NULL OR LOWER(p.category) = LOWER(:category))
             AND (:minPrice IS NULL OR p.price >= :minPrice)
             AND (:maxPrice IS NULL OR p.price <= :maxPrice)
           """)
    Page<Product> searchProducts(@Param("keyword") String keyword,
                                 @Param("category") String category,
                                 @Param("minPrice") BigDecimal minPrice,
                                 @Param("maxPrice") BigDecimal maxPrice,
                                 Pageable pageable);
}</gap:target>

<gap:target id="dto-section">
/**
 * Request DTO for product data.
 */
public class ProductRequest {

    @NotBlank
    private String name;

    @NotBlank
    private String sku;

    private String description;

    @NotBlank
    private String category;

    @NotNull
    @DecimalMin("0.0")
    private BigDecimal price;

    @NotNull
    @Min(0)
    private Integer stockQuantity;

    @NotNull
    private Boolean active;

    public String getName() { return name; }
    public void setName(String name) { this.name = name; }

    public String getSku() { return sku; }
    public void setSku(String sku) { this.sku = sku; }

    public String getDescription() { return description; }
    public void setDescription(String description) { this.description = description; }

    public String getCategory() { return category; }
    public void setCategory(String category) { this.category = category; }

    public BigDecimal getPrice() { return price; }
    public void setPrice(BigDecimal price) { this.price = price; }

    public Integer getStockQuantity() { return stockQuantity; }
    public void setStockQuantity(Integer stockQuantity) { this.stockQuantity = stockQuantity; }

    public Boolean getActive() { return active; }
    public void setActive(Boolean active) { this.active = active; }
}

/**
 * Response DTO for product data.
 */
public class ProductResponse {

    private Long id;
    private String name;
    private String sku;
    private String description;
    private String category;
    private BigDecimal price;
    private Integer stockQuantity;
    private Boolean active;
    private LocalDateTime createdAt;
    private LocalDateTime updatedAt;

    public ProductResponse(Long id, String name, String sku, String description, String category,
                           BigDecimal price, Integer stockQuantity, Boolean active,
                           LocalDateTime createdAt, LocalDateTime updatedAt) {
        this.id = id;
        this.name = name;
        this.sku = sku;
        this.description = description;
        this.category = category;
        this.price = price;
        this.stockQuantity = stockQuantity;
        this.active = active;
        this.createdAt = createdAt;
        this.updatedAt = updatedAt;
    }

    public Long getId() { return id; }
    public String getName() { return name; }
    public String getSku() { return sku; }
    public String getDescription() { return description; }
    public String getCategory() { return category; }
    public BigDecimal getPrice() { return price; }
    public Integer getStockQuantity() { return stockQuantity; }
    public Boolean getActive() { return active; }
    public LocalDateTime getCreatedAt() { return createdAt; }
    public LocalDateTime getUpdatedAt() { return updatedAt; }
}

/**
 * Search criteria for products.
 */
public class ProductSearchCriteria {

    private String keyword;
    private String category;
    private BigDecimal minPrice;
    private BigDecimal maxPrice;

    @Min(0)
    private int page = 0;

    @Min(1)
    private int size = 10;

    private String sortBy = "name";
    private String sortDir = "asc";

    public String getKeyword() { return keyword; }
    public void setKeyword(String keyword) { this.keyword = keyword; }

    public String getCategory() { return category; }
    public void setCategory(String category) { this.category = category; }

    public BigDecimal getMinPrice() { return minPrice; }
    public void setMinPrice(BigDecimal minPrice) { this.minPrice = minPrice; }

    public BigDecimal getMaxPrice() { return maxPrice; }
    public void setMaxPrice(BigDecimal maxPrice) { this.maxPrice = maxPrice; }

    public int getPage() { return page; }
    public void setPage(int page) { this.page = page; }

    public int getSize() { return size; }
    public void setSize(int size) { this.size = size; }

    public String getSortBy() { return sortBy; }
    public void setSortBy(String sortBy) { this.sortBy = sortBy; }

    public String getSortDir() { return sortDir; }
    public void setSortDir(String sortDir) { this.sortDir = sortDir; }
}

/**
 * Generic paged response wrapper.
 *
 * @param <T> response content type
 */
public class PagedResponse<T> {

    private List<T> content;
    private int page;
    private int size;
    private long totalElements;
    private int totalPages;
    private boolean last;

    public PagedResponse(List<T> content, int page, int size, long totalElements, int totalPages, boolean last) {
        this.content = content;
        this.page = page;
        this.size = size;
        this.totalElements = totalElements;
        this.totalPages = totalPages;
        this.last = last;
    }

    public List<T> getContent() { return content; }
    public int getPage() { return page; }
    public int getSize() { return size; }
    public long getTotalElements() { return totalElements; }
    public int getTotalPages() { return totalPages; }
    public boolean isLast() { return last; }
}
</gap:target>

<gap:target id="exception-section">
/**
 * Global exception handler for REST APIs.
 */
@ControllerAdvice
public class GlobalExceptionHandler {

    @ExceptionHandler(ResourceNotFoundException.class)
    public ResponseEntity<ErrorResponse> handleNotFound(ResourceNotFoundException ex) {
        return ResponseEntity.status(HttpStatus.NOT_FOUND)
                .body(new ErrorResponse("<gap:target id="not-found-error-code">NOT_FOUND</gap:target>", ex.getMessage()));
    }

    @ExceptionHandler(BadRequestException.class)
    public ResponseEntity<ErrorResponse> handleBadRequest(BadRequestException ex) {
        return ResponseEntity.status(HttpStatus.BAD_REQUEST)
                .body(new ErrorResponse("BAD_REQUEST", ex.getMessage()));
    }

    @ExceptionHandler(MethodArgumentNotValidException.class)
    public ResponseEntity<ErrorResponse> handleValidation(MethodArgumentNotValidException ex) {
        String message = ex.getBindingResult().getFieldErrors()
                .stream()
                .map(err -> err.getField() + ": " + err.getDefaultMessage())
                .findFirst()
                .orElse("Validation failed");
        return ResponseEntity.status(HttpStatus.BAD_REQUEST)
                .body(new ErrorResponse("VALIDATION_ERROR", message));
    }

    @ExceptionHandler(Exception.class)
    public ResponseEntity<ErrorResponse> handleGeneric(Exception ex) {
        return ResponseEntity.status(HttpStatus.INTERNAL_SERVER_ERROR)
                .body(new ErrorResponse("INTERNAL_SERVER_ERROR", "An unexpected error occurred"));
    }
}

/**
 * Error response body.
 */
public class ErrorResponse {
    private String code;
    private String message;

    public ErrorResponse(String code, String message) {
        this.code = code;
        this.message = message;
    }

    public String getCode() { return code; }
    public String getMessage() { return message; }
}

/**
 * Resource not found exception.
 */
public class ResourceNotFoundException extends RuntimeException {
    public ResourceNotFoundException(String message) {
        super(message);
    }
}

/**
 * Bad request exception.
 */
public class BadRequestException extends RuntimeException {
    public BadRequestException(String message) {
        super(message);
    }
}
</gap:target>

<gap:target id="entity-section">
@Entity
@Table(name = "products")
public class Product {

    @Id
    @GeneratedValue(strategy = GenerationType.IDENTITY)
    private Long id;

    private String name;
    private String sku;

    @Column(length = 2000)
    private String description;

    private String category;

    @Column(precision = 19, scale = 2)
    private BigDecimal price;

    private Integer stockQuantity;
    private Boolean active;

    @CreationTimestamp
    private LocalDateTime createdAt;

    @UpdateTimestamp
    private LocalDateTime updatedAt;

    public Long getId() { return id; }
    public String getName() { return name; }
    public void setName(String name) { this.name = name; }
    public String getSku() { return sku; }
    public void setSku(String sku) { this.sku = sku; }
    public String getDescription() { return description; }
    public void setDescription(String description) { this.description = description; }
    public String getCategory() { return category; }
    public void setCategory(String category) { this.category = category; }
    public BigDecimal getPrice() { return price; }
    public void setPrice(BigDecimal price) { this.price = price; }
    public Integer getStockQuantity() { return stockQuantity; }
    public void setStockQuantity(Integer stockQuantity) { this.stockQuantity = stockQuantity; }
    public Boolean getActive() { return active; }
    public void setActive(Boolean active) { this.active = active; }
    public LocalDateTime getCreatedAt() { return createdAt; }
    public LocalDateTime getUpdatedAt() { return updatedAt; }
}
</gap:target>

</gap:target>