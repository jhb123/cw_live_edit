var signUpDialog = document.getElementById("signUpDialog");
var showSignUpDialog = document.getElementById("showSignUpDialog")
var closeSignUpDialog = document.getElementById("closeSignUpDialog");

showSignUpDialog.addEventListener("click", () => {
    signUpDialog.showModal();
});

closeSignUpDialog.addEventListener("click", () => {
    signUpDialog.close();
});

var logInDialog = document.getElementById("logInDialog");
var showLogInDialog = document.getElementById("showLogInDialog")
var closeLogInDialog = document.getElementById("closeLogInDialog");

showLogInDialog.addEventListener("click", () => {
    logInDialog.showModal();
});

closeLogInDialog.addEventListener("click", () => {
    logInDialog.close();
});