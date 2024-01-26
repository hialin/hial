package main

import (
	"encoding/json"
	"fmt"
	"log"
	"net/http"
)

// User struct to demonstrate JSON encoding/decoding
type User struct {
	ID        int    `json:"id"`
	FirstName string `json:"firstName"`
	LastName  string `json:"lastName"`
	Email     string `json:"email"`
}

// Users slice to store multiple users
var Users []User

// HomePage function to handle requests on the root
func HomePage(w http.ResponseWriter, r *http.Request) {
	fmt.Fprintf(w, "Welcome to the HomePage!")
	log.Println("Endpoint Hit: HomePage")
}

// ReturnAllUsers function to return all users in JSON format
func ReturnAllUsers(w http.ResponseWriter, r *http.Request) {
	fmt.Println("Endpoint Hit: returnAllUsers")
	json.NewEncoder(w).Encode(Users)
}

// CreateUser function to handle POST requests and add a new user
func CreateUser(w http.ResponseWriter, r *http.Request) {
	fmt.Println("Endpoint Hit: createUser")

	// Decode the incoming User json
	var newUser User
	err := json.NewDecoder(r.Body).Decode(&newUser)
	if err != nil {
		http.Error(w, err.Error(), http.StatusBadRequest)
		return
	}

	// Append to the Users slice
	Users = append(Users, newUser)

	json.NewEncoder(w).Encode(newUser)
}

// SetupRoutes sets up all the server routes
func SetupRoutes() {
	http.HandleFunc("/", HomePage)
	http.HandleFunc("/users", ReturnAllUsers)
	http.HandleFunc("/user", CreateUser)
}

// Initialize data
func init() {
	Users = []User{
		User{ID: 1, FirstName: "John", LastName: "Doe", Email: "john@example.com"},
		User{ID: 2, FirstName: "Jane", LastName: "Doe", Email: "jane@example.com"},
	}
}

func main() {
	// Setup the HTTP server routes
	SetupRoutes()

	// Start the server and log errors, if any
	log.Fatal(http.ListenAndServe(":8080", nil))
}
