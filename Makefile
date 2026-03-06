.PHONY: test-docker test-docker-keep test-docker-clean test-docker-logs help

# Run all tests in Docker
test-docker:
	@./scripts/test-docker.sh

# Run tests and keep containers
test-docker-keep:
	@./scripts/test-docker.sh --keep

# Clean Docker containers and volumes
test-docker-clean:
	@docker-compose -f docker-compose.test.yml down -v

# View service logs
test-docker-logs:
	@docker-compose -f docker-compose.test.yml logs -f

# Help
help:
	@echo "Available commands:"
	@echo "  make test-docker        - Run all tests in Docker"
	@echo "  make test-docker-keep   - Run tests and keep containers"
	@echo "  make test-docker-clean  - Clean containers and volumes"
	@echo "  make test-docker-logs   - View service logs"
	@echo "  make help               - Show this help"
